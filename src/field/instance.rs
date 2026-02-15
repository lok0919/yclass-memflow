use super::{
    create_text_format, display_field_name, display_field_prelude, display_field_value, next_id,
    CodegenData, Field, FieldId, FieldKind, FieldResponse, NamedState,
};
use crate::context::InspectionContext;
use crate::generator::Generator;
use eframe::{
    egui::{collapsing_header::CollapsingState, Id, Label, RichText, Sense, TextFormat, Ui},
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
            (format!("[Instance]"), false)
        };

        let mut job = LayoutJob::default();
        let is_misaligned = display_field_prelude(ui.ctx(), self, ctx, &mut job);
        job.append(" ", 0., TextFormat::default());

        let r = ui.add(Label::new(job).sense(Sense::click()));
        let clicked = r.clicked();
        if is_misaligned {
            r.on_hover_text(format!(
                "Misaligned: {}-byte field at offset {:04X} (not {}-byte aligned)",
                self.size(),
                ctx.offset,
                self.size()
            ));
        }
        if clicked {
            ctx.select(self.id);
        }

        display_field_name(self, ui, ctx, &self.state, Color32::BROWN);

        let is_selected = ctx.is_selected(self.id);

        ui.add_space(4.);

        display_field_value(
            self,
            ui,
            ctx,
            &self.state,
            Color32::YELLOW,
            |_| format!("{} bytes", self.size()),
            |_| false,
        );

        let mut job = LayoutJob::default();
        job.append(
            &text,
            4.,
            create_text_format(
                is_selected,
                if exists {
                    Color32::LIGHT_GRAY
                } else {
                    Color32::DARK_GRAY
                },
            ),
        );

        let r = ui.add(Label::new(job).sense(Sense::click()));
        if r.secondary_clicked() {
            ui.memory_mut(|m| m.toggle_popup(Id::new(ctx.current_id)));
        } else if r.clicked() {
            ctx.select(self.id);
        }

        // Show popup for selecting class
        use eframe::egui::popup_below_widget;
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
                // Will be immediately reassigned.
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
        64
    }

    fn name(&self) -> Option<String> {
        Some(self.state.name.borrow().clone())
    }

    fn kind(&self) -> FieldKind {
        FieldKind::Instance
    }

    fn draw(&self, ui: &mut Ui, ctx: &mut InspectionContext) -> Option<FieldResponse> {
        let mut response = None;

        if self.class_id.get().is_none() {
            self.class_id.set(Some(fastrand::usize(..)));
        }

        let state = CollapsingState::load_with_default_open(ui.ctx(), ctx.current_id, false);
        let body = state
            .show_header(ui, |ui| self.show_header(ui, ctx))
            .body(|ui| self.show_body(ui, ctx))
            .2;
        let body = body.and_then(|inner| inner.inner);

        if let Some(new) = body {
            response = Some(new);
        }

        ctx.offset += self.size();
        response
    }

    fn codegen(&self, generator: &mut dyn Generator, data: &CodegenData) {
        generator.add_field(
            self.state.name.borrow().as_str(),
            FieldKind::Instance,
            data.classes
                .iter()
                .find(|c| c.id() == self.class_id.get().unwrap())
                .map(|c| c.name.as_ref()),
        );
    }
}
