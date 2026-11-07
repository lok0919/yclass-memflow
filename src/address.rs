//! This module parses addresses and manages address expressions.
//! All numbers are parsed as hexadecimals.
//! Supported operations: `+`, `-`, `*`, `/`, and parentheses `()`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    expression: String,
    value: usize,
}

impl Default for Address {
    fn default() -> Self {
        Self::from_value(0)
    }
}

impl Address {
    pub fn new(expression: impl Into<String>) -> Option<Self> {
        let expression = expression.into();
        let value = parse_address(&expression)?;
        Some(Self { expression, value })
    }

    pub fn from_value(value: usize) -> Self {
        Self {
            expression: format!("0x{value:X}"),
            value,
        }
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }

    pub fn value(&self) -> usize {
        self.value
    }

    pub fn set_value(&mut self, value: usize) {
        self.value = value;
        self.expression = format!("0x{value:X}");
    }

    pub fn set_expression(&mut self, expr: impl Into<String>) -> bool {
        let expr = expr.into();
        if let Some(val) = parse_address(&expr) {
            self.expression = expr;
            self.value = val;
            true
        } else {
            false
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expression)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Token {
    Hex(usize),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            _ => {
                if i + 1 < chars.len()
                    && chars[i] == '0'
                    && (chars[i + 1] == 'x' || chars[i + 1] == 'X')
                {
                    i += 2;
                }

                let num_start = i;
                while i < chars.len() && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }

                if i == num_start {
                    return None;
                }

                let num_str: String = chars[num_start..i].iter().collect();
                let val = usize::from_str_radix(&num_str, 16).ok()?;
                tokens.push(Token::Hex(val));
            }
        }
    }

    if tokens.is_empty() {
        return None;
    }

    Some(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.peek()?;
        self.pos += 1;
        Some(tok)
    }

    fn parse_expr(&mut self) -> Option<usize> {
        let mut left = self.parse_term()?;

        while let Some(op) = self.peek() {
            match op {
                Token::Plus => {
                    self.next();
                    let right = self.parse_term()?;
                    left = left.wrapping_add(right);
                }
                Token::Minus => {
                    self.next();
                    let right = self.parse_term()?;
                    left = left.wrapping_sub(right);
                }
                _ => break,
            }
        }

        Some(left)
    }

    fn parse_term(&mut self) -> Option<usize> {
        let mut left = self.parse_factor()?;

        while let Some(op) = self.peek() {
            match op {
                Token::Star => {
                    self.next();
                    let right = self.parse_factor()?;
                    left = left.wrapping_mul(right);
                }
                Token::Slash => {
                    self.next();
                    let right = self.parse_factor()?;
                    if right == 0 {
                        return None;
                    }
                    left = left.checked_div(right)?;
                }
                _ => break,
            }
        }

        Some(left)
    }

    fn parse_factor(&mut self) -> Option<usize> {
        let mut sign = 1i32;

        while let Some(op) = self.peek() {
            match op {
                Token::Plus => {
                    self.next();
                }
                Token::Minus => {
                    self.next();
                    sign = -sign;
                }
                _ => break,
            }
        }

        let val = self.parse_primary()?;
        if sign == -1 {
            Some(0usize.wrapping_sub(val))
        } else {
            Some(val)
        }
    }

    fn parse_primary(&mut self) -> Option<usize> {
        match self.next()? {
            Token::Hex(val) => Some(val),
            Token::LParen => {
                let val = self.parse_expr()?;
                if self.next() != Some(Token::RParen) {
                    return None;
                }
                Some(val)
            }
            _ => None,
        }
    }
}

pub fn parse_address(addr: &str) -> Option<usize> {
    let tokens = tokenize(addr)?;
    let mut parser = Parser::new(&tokens);
    let result = parser.parse_expr()?;
    if parser.pos == tokens.len() {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_hex() {
        assert_eq!(parse_address("0"), Some(0));
        assert_eq!(parse_address("0x0"), Some(0));
        assert_eq!(parse_address("0x10"), Some(16));
        assert_eq!(parse_address("10"), Some(16));
        assert_eq!(parse_address("0xFF"), Some(255));
        assert_eq!(parse_address("0Xff"), Some(255));
        assert_eq!(parse_address("  1A  "), Some(26));
    }

    #[test]
    fn test_addition_subtraction() {
        assert_eq!(parse_address("0x10 + 0x20"), Some(48));
        assert_eq!(parse_address("0x20 - 0x10"), Some(16));
        assert_eq!(parse_address("10 + 20"), Some(48));
        assert_eq!(parse_address("0x100 - 0x1"), Some(255));
    }

    #[test]
    fn test_multiplication_division() {
        assert_eq!(parse_address("0x10 * 0x2"), Some(32));
        assert_eq!(parse_address("0x20 / 0x4"), Some(8));
        assert_eq!(parse_address("0x10 / 0"), None);
    }

    #[test]
    fn test_precedence() {
        assert_eq!(parse_address("0x10 + 0x2 * 0x3"), Some(16 + 6));
        assert_eq!(parse_address("(0x10 + 0x2) * 0x3"), Some((16 + 2) * 3));
        assert_eq!(parse_address("0x30 / 0x2 + 0x5"), Some(24 + 5));
    }

    #[test]
    fn test_unary_signs() {
        assert_eq!(parse_address("+0x10"), Some(16));
        assert_eq!(parse_address("0x20 + -0x10"), Some(16));
    }

    #[test]
    fn test_invalid_syntax() {
        assert_eq!(parse_address(""), None);
        assert_eq!(parse_address("   "), None);
        assert_eq!(parse_address("0x"), None);
        assert_eq!(parse_address("0x+"), None);
        assert_eq!(parse_address("0x10 +"), None);
        assert_eq!(parse_address("+"), None);
        assert_eq!(parse_address("(0x10"), None);
        assert_eq!(parse_address("0x10)"), None);
        assert_eq!(parse_address("hello"), None);
    }

    #[test]
    fn test_address_struct() {
        let mut addr = Address::new("0x1000 + 0x20").unwrap();
        assert_eq!(addr.value(), 0x1020);
        assert_eq!(addr.expression(), "0x1000 + 0x20");

        assert!(addr.set_expression("0x2000 + 0x4 * 0x2"));
        assert_eq!(addr.value(), 0x2008);
        assert_eq!(addr.expression(), "0x2000 + 0x4 * 0x2");

        assert!(!addr.set_expression("invalid syntax"));
        assert_eq!(addr.value(), 0x2008);
        assert_eq!(addr.expression(), "0x2000 + 0x4 * 0x2");

        addr.set_value(0x3000);
        assert_eq!(addr.value(), 0x3000);
        assert_eq!(addr.expression(), "0x3000");
        assert_eq!(addr.to_string(), "0x3000");
    }
}
