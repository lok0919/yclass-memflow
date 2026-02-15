use super::{
    create_text_format, display_field_name, display_field_prelude, next_id, CodegenData, Field,
    FieldId, FieldKind, FieldResponse, NamedState,
};
use crate::context::InspectionContext;
use crate::generator::Generator;
use eframe::{
    egui::{
        collapsing_header::CollapsingState, popup_below_widget, Id, Label, RichText, Sense,
        TextFormat, Ui,
    },
    epaint::{text::LayoutJob, Color32},
};
use fastrand::Rng;
use std::{cell::Cell, mem::transmute};

pub struct InstanceField {
    id: FieldId,
    state: NamedState,
    class_id: Cell<Option<usize>>,
}

impl InstanceField {
    pub fn new(name: String) -> Self {
        Self {
            id: next_id(),
            state: NamedState::new(name),
            class_id: None.into(),
        }
    }

    pub fn new_with_class_id(name: String, class_id: usize) -> Self {
        Self {
            id: next_id(),
            state: NamedState::new(name),
            class_id: Some(class_id).into(),
        }
    }

    fn show_header(&self, ui: &mut Ui, ctx: &mut InspectionContext) {
        let class = self.class_id.get().and_then(|id| ctx.class_list.by_id(id));

        let (text, exists) = if let Some(cl) = class {
            (format!("[{}]", cl.name), true)
        } else {
            (format!("[unassigned]"), false)
        };

        let mut job = LayoutJob::default();
        let _is_misaligned = display_field_prelude(ui.ctx(), self, ctx, &mut job);
        job.append(" ", 0., TextFormat::default());

        let r = ui.add(Label::new(job).sense(Sense::click()));
        if r.clicked() {
            ctx.select(self.id);
        }

        display_field_name(self, ui, ctx, &self.state, Color32::BROWN);

        let is_selected = ctx.is_selected(self.id);

        ui.add_space(4.);

        // Display class reference with right-click popup
        let mut job = LayoutJob::default();
        job.append(
            &text,
            4.,
            create_text_format(
                is_selected,
                if exists {
                    Color32::LIGHT_GRAY
                } else {
                    Color32::DARK_RED
                },
            ),
        );

        let r = ui.add(Label::new(job).sense(Sense::click()));
        if r.secondary_clicked() {
            ui.memory_mut(|m| m.toggle_popup(Id::new(ctx.current_id)));
        } else if r.clicked() {
            ctx.select(self.id);
        }

        popup_below_widget(ui, Id::new(ctx.current_id), &r, |ui| {
            ui.set_width(80.);
            ui.vertical_centered_justified(|ui| {
                for cl in ctx.class_list.classes() {
                    if ui.button(&cl.name).clicked() {
                        self.class_id.set(Some(cl.id()));
                    }
                }
            });
        });
    }

    fn show_body(&self, ui: &mut Ui, ctx: &mut InspectionContext) -> Option<FieldResponse> {
        // If no class is assigned, show a message
        if self.class_id.get().is_none() {
            ui.label(
                RichText::new("Right-click the class reference above to assign a class")
                    .color(Color32::LIGHT_GRAY),
            );
            return None;
        }

        if !ctx.process.can_read(ctx.address + ctx.offset) {
            ui.heading(
                RichText::new(format!(
                    "Can't read memory at address {:#X}",
                    ctx.address + ctx.offset
                ))
                .color(Color32::RED),
            );
            return None;
        }

        let mut response = None;

        let cid = self.class_id.get()?;
        if let Some(class) = ctx.class_list.by_id(cid) {
            let rng = Rng::with_seed(unsafe { transmute(ctx.current_id) });

            let mut inner_ctx = InspectionContext {
                class_list: ctx.class_list,
                parent_id: ctx.current_id,
                selection: ctx.selection,
                current_container: cid,
                current_id: Id::null(),
                process: ctx.process,
                toasts: ctx.toasts,
                level_rng: &rng,
                offset: 0,
                address: ctx.address + ctx.offset,
            };

            #[allow(clippy::single_match)]
            match class.fields.iter().fold(None, |r, f| {
                inner_ctx.current_id = Id::new(rng.u64(..));
                r.or(f.draw(ui, &mut inner_ctx))
            }) {
                Some(other) => response = Some(other),
                None => {}
            }

            ctx.selection = inner_ctx.selection;
        }

        response
    }
}

impl Field for InstanceField {
    fn id(&self) -> FieldId {
        self.id
    }

    fn size(&self) -> usize {
        // Calculate size from embedded class
        self.class_id
            .get()
            .and_then(|_| {
                // We can't access context here, so return a placeholder
                // The actual size is calculated when rendering
                None
            })
            .unwrap_or(0)
        // If we can't determine size (no class assigned), return 0
        // This will be overridden during draw() where we have context
    }

    fn name(&self) -> Option<String> {
        Some(self.state.name.borrow().clone())
    }

    fn kind(&self) -> FieldKind {
        FieldKind::Instance
    }

    fn draw(&self, ui: &mut Ui, ctx: &mut InspectionContext) -> Option<FieldResponse> {
        // Calculate actual size from the embedded class
        let actual_size = self
            .class_id
            .get()
            .and_then(|id| ctx.class_list.by_id(id))
            .map(|cl| cl.size())
            .unwrap_or(0);

        let mut response = None;

        let state = CollapsingState::load_with_default_open(ui.ctx(), ctx.current_id, false);
        let body = state
            .show_header(ui, |ui| self.show_header(ui, ctx))
            .body(|ui| self.show_body(ui, ctx))
            .2;
        let body = body.and_then(|inner| inner.inner);

        if let Some(new) = body {
            response = Some(new);
        }

        // Only advance offset if a class is assigned
        if actual_size > 0 {
            ctx.offset += actual_size;
        }
        response
    }

    fn codegen(&self, generator: &mut dyn Generator, data: &CodegenData) {
        if let Some(class_id) = self.class_id.get() {
            if let Some(class) = data.classes.iter().find(|c| c.id() == class_id) {
                generator.add_field(
                    self.state.name.borrow().as_str(),
                    FieldKind::Instance,
                    Some(class.name.as_ref()),
                );
            }
        }
    }
}
