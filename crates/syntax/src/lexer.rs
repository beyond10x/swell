//! The lexer (`#sec-ecmascript-language-lexical-grammar`), over UTF-16 code units.
//!
//! It reads a hashbang comment at the very start of the source (`#sec-hashbang`), white space
//! (`#sec-white-space`, `USP` through `swell_unicode::is_space_separator`), line terminators
//! (`#sec-line-terminators`), comments (`#sec-comments`), ASCII identifier names, with `\u`
//! escapes whose code point is ASCII (`#sec-identifier-names`), and reserved words
//! (`#sec-keywords-and-reserved-words`: a name written with an escape is never a keyword),
//! punctuators (`#sec-punctuators`), numeric literals with their separators
//! (`#sec-literals-numeric-literals`), string literals with every escape
//! (`#sec-literals-string-literals`), the template tokens `NoSubstitutionTemplate` and
//! `TemplateHead` and, when the parser asks for the `InputElementTemplateTail` goal at the `}`
//! that ends a substitution, `TemplateMiddle` and `TemplateTail`
//! (`#sec-template-literal-lexical-components`), each with its TV and TRV, and, when the parser
//! asks for the `InputElementRegExp` goal, regular expression literals
//! (`#sec-literals-regular-expression-literals`).
//!
//! Refused here, each with the slice that brings it: an identifier with a code point outside
//! ASCII, written or through a `\u` escape (5, a recorded omission of `sec-names-and-keywords`),
//! private names (8), BigInt literals (12) and the HTML-like comments of Annex B
//! (`#sec-html-like-comments`, 18).
//!
//! The lexer runs once over the source and every loop advances by at least one code unit, so its
//! work is linear in the length of the source; a part lexed again at the parser's request (a
//! regular expression literal, the characters of a template after a `}`) was lexed before only
//! as the one punctuator at its start. It runs before any script does; the step and memory
//! budgets of `docs/design/engine.md` decision 3 count work done for a running script.

use crate::ast::{NumberForm, NumberLiteral, RegExpLiteral, StringLiteral, TemplateString};
use crate::error::SyntaxError;
use crate::token::{Keyword, Punct, TemplatePart, Token, TokenKind};

/// A template token with no closing `` ` `` or `${` before the end of the source.
const UNTERMINATED_TEMPLATE: &str = "unterminated template literal";

/// `#!` anywhere but at the very start of the source (`#sec-hashbang`).
const HASHBANG_NOT_AT_START: &str = "hashbang comment not at the start of the source";

/// A `NumericLiteralSeparator` with no digit on one side of it: at the end of the digits, next to
/// `.`, `e`, a sign or a prefix (`#sec-literals-numeric-literals`, `DecimalDigits[+Sep]`).
const SEPARATOR_NOT_BETWEEN_DIGITS: &str = "numeric separator not between two digits";

/// Two `NumericLiteralSeparator`s in a row.
const SEPARATOR_DOUBLED: &str = "numeric separator doubled";

/// A `NumericLiteralSeparator` after the leading `0` of `0`, a `LegacyOctalIntegerLiteral` or a
/// `NonOctalDecimalIntegerLiteral`, none of which take one.
const SEPARATOR_AFTER_LEADING_ZERO: &str = "numeric separator after a leading 0";

const LF: u16 = 0x0A;
const CR: u16 = 0x0D;
const LS: u16 = 0x2028;
const PS: u16 = 0x2029;

fn is_line_terminator(c: u16) -> bool {
    matches!(c, LF | CR | LS | PS)
}

/// `WhiteSpace` (`#sec-white-space`): TAB, VT, FF, ZWNBSP and every `Zs` code point. Every `Zs`
/// code point is in the Basic Multilingual Plane, so one code unit decides.
fn is_white_space(c: u16) -> bool {
    matches!(c, 0x09 | 0x0B | 0x0C | 0x20 | 0xFEFF)
        || (c >= 0x80 && swell_unicode::is_space_separator(u32::from(c)))
}

fn is_ascii_id_start(c: u16) -> bool {
    matches!(c, 0x41..=0x5A | 0x61..=0x7A | 0x24 | 0x5F)
}

fn is_ascii_id_part(c: u16) -> bool {
    is_ascii_id_start(c) || is_decimal(c)
}

fn is_decimal(c: u16) -> bool {
    (0x30..=0x39).contains(&c)
}

fn is_octal(c: u16) -> bool {
    (0x30..=0x37).contains(&c)
}

fn hex_value(c: u16) -> Option<u32> {
    match c {
        0x30..=0x39 => Some(u32::from(c - 0x30)),
        0x41..=0x46 => Some(u32::from(c - 0x41 + 10)),
        0x61..=0x66 => Some(u32::from(c - 0x61 + 10)),
        _ => None,
    }
}

fn digit_of_radix(c: u16, radix: u32) -> bool {
    match radix {
        2 => matches!(c, 0x30 | 0x31),
        8 => is_octal(c),
        16 => hex_value(c).is_some(),
        _ => is_decimal(c),
    }
}

fn ascii(c: u8) -> u16 {
    u16::from(c)
}

/// Appends the UTF-16 encoding of the code point `cp` (at most 0x10FFFF) to `out`.
fn push_code_point(out: &mut Vec<u16>, cp: u32) {
    if cp < 0x1_0000 {
        out.push(u16::try_from(cp).unwrap_or(0xFFFD));
    } else {
        let v = cp - 0x1_0000;
        out.push(u16::try_from(0xD800 + (v >> 10)).unwrap_or(0xFFFD));
        out.push(u16::try_from(0xDC00 + (v & 0x3FF)).unwrap_or(0xFFFD));
    }
}

/// The TRV of template characters (`#sec-static-semantics-trv`): their source text, with each
/// `<CR><LF>` and each `<CR>` a `<LF>`.
fn trv(units: &[u16]) -> Vec<u16> {
    let mut raw = Vec::with_capacity(units.len());
    let mut rest = units.iter().copied().peekable();
    while let Some(u) = rest.next() {
        if u == CR {
            if rest.peek() == Some(&LF) {
                rest.next();
            }
            raw.push(LF);
        } else {
            raw.push(u);
        }
    }
    raw
}

pub(crate) struct Lexer<'a> {
    src: &'a [u16],
    pos: usize,
    line: u32,
    line_start: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(src: &'a [u16]) -> Lexer<'a> {
        Lexer {
            src,
            pos: 0,
            line: 1,
            line_start: 0,
        }
    }

    fn unit(&self, at: usize) -> Option<u16> {
        self.src.get(at).copied()
    }

    fn cur(&self) -> Option<u16> {
        self.unit(self.pos)
    }

    fn ahead(&self, n: usize) -> Option<u16> {
        self.unit(self.pos.saturating_add(n))
    }

    fn column(&self) -> u32 {
        u32::try_from(self.pos.saturating_sub(self.line_start) + 1).unwrap_or(u32::MAX)
    }

    fn error(&self, message: impl Into<String>) -> SyntaxError {
        SyntaxError::new(message, self.line, self.column())
    }

    fn refused(&self, construct: &str, slice: u32) -> SyntaxError {
        SyntaxError::refused(construct, slice, self.line, self.column())
    }

    /// Consumes the line terminator at the current position (CR LF as one,
    /// `LineTerminatorSequence`) and starts a new line.
    fn line_terminator(&mut self) {
        if self.cur() == Some(CR) && self.ahead(1) == Some(LF) {
            self.pos += 2;
        } else {
            self.pos += 1;
        }
        self.line = self.line.saturating_add(1);
        self.line_start = self.pos;
    }

    fn offset(at: usize) -> u32 {
        u32::try_from(at).unwrap_or(u32::MAX)
    }

    /// The `/*` comment at the current position; whether it held a line terminator.
    fn multi_line_comment(&mut self) -> Result<bool, SyntaxError> {
        let (line, column) = (self.line, self.column());
        self.pos += 2;
        let mut newline = false;
        loop {
            match self.cur() {
                None => return Err(SyntaxError::new("unterminated comment", line, column)),
                Some(0x2A) if self.ahead(1) == Some(0x2F) => {
                    self.pos += 2;
                    return Ok(newline);
                }
                Some(c) if is_line_terminator(c) => {
                    newline = true;
                    self.line_terminator();
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    /// Skips white space, line terminators and comments; whether a line terminator was among
    /// them (`#sec-comments`: a `MultiLineComment` with one counts as one).
    fn skip_trivia(&mut self) -> Result<bool, SyntaxError> {
        if self.pos == 0 && self.cur() == Some(ascii(b'#')) && self.ahead(1) == Some(ascii(b'!')) {
            // `HashbangComment :: #! SingleLineCommentChars?`, up to the line terminator.
            while self.cur().is_some_and(|c| !is_line_terminator(c)) {
                self.pos += 1;
            }
        }
        let mut newline = false;
        while let Some(c) = self.cur() {
            if is_line_terminator(c) {
                newline = true;
                self.line_terminator();
            } else if is_white_space(c) {
                self.pos += 1;
            } else if c == ascii(b'/') && self.ahead(1) == Some(ascii(b'/')) {
                while self.cur().is_some_and(|c| !is_line_terminator(c)) {
                    self.pos += 1;
                }
            } else if c == ascii(b'/') && self.ahead(1) == Some(ascii(b'*')) {
                if self.multi_line_comment()? {
                    newline = true;
                }
            } else if (c == ascii(b'<')
                && self.ahead(1) == Some(ascii(b'!'))
                && self.ahead(2) == Some(ascii(b'-'))
                && self.ahead(3) == Some(ascii(b'-')))
                || (c == ascii(b'-')
                    && (newline || self.pos == 0)
                    && self.ahead(1) == Some(ascii(b'-'))
                    && self.ahead(2) == Some(ascii(b'>')))
            {
                return Err(self.refused("HTML-like comment", 18));
            } else {
                break;
            }
        }
        Ok(newline)
    }

    /// The next token in the `InputElementDiv` goal: a `/` is a punctuator. The parser asks for a
    /// regular expression literal with [`Lexer::rescan_regexp`].
    pub(crate) fn next_token(&mut self) -> Result<Token, SyntaxError> {
        let newline_before = self.skip_trivia()?;
        let start = self.pos;
        let (line, column) = (self.line, self.column());
        let mut escaped = false;
        let kind = match self.cur() {
            None => TokenKind::Eof,
            Some(c) if is_ascii_id_start(c) || c == ascii(b'\\') => {
                let (kind, with_escape) = self.identifier_name()?;
                escaped = with_escape;
                kind
            }
            Some(c) => self.token_kind(c)?,
        };
        Ok(Token {
            kind,
            start: Lexer::offset(start),
            end: Lexer::offset(self.pos),
            line,
            column,
            newline_before,
            escaped,
        })
    }

    fn token_kind(&mut self, c: u16) -> Result<TokenKind, SyntaxError> {
        if is_decimal(c) || (c == ascii(b'.') && self.ahead(1).is_some_and(is_decimal)) {
            return self.number();
        }
        if c == ascii(b'"') || c == ascii(b'\'') {
            return self.string(c).map(TokenKind::String);
        }
        if c == ascii(b'`') {
            return self.template();
        }
        if c == ascii(b'#') {
            return Err(if self.ahead(1) == Some(ascii(b'!')) {
                self.error(HASHBANG_NOT_AT_START)
            } else if self.ahead(1).is_some_and(is_ascii_id_start) {
                self.refused("private name", 8)
            } else {
                self.error("unexpected character #")
            });
        }
        if c >= 0x80 {
            // White space and line terminators are skipped already; what remains outside ASCII
            // can only start an identifier, whose tables are slice 5's.
            return Err(self.refused("identifier with a code point outside ASCII", 5));
        }
        match self.punctuator(c) {
            Some(p) => Ok(TokenKind::Punct(p)),
            None => Err(self.error(format!("unexpected character U+{:04X}", u32::from(c)))),
        }
    }

    /// An `IdentifierName` (`#sec-identifier-names`) at the current position: ASCII code points
    /// and `\ UnicodeEscapeSequence`s, each of whose code point must be one the place allows
    /// (`#sec-identifier-names-static-semantics-early-errors`: an `IdentifierStartChar` first,
    /// an `IdentifierPartChar` after). Answers the token and whether an escape was among it: a
    /// name with an escape is a [`TokenKind::Name`] whatever its StringValue, never a keyword.
    /// A code point outside ASCII, written or escaped, is refused (slice 5's tables).
    fn identifier_name(&mut self) -> Result<(TokenKind, bool), SyntaxError> {
        let mut name = String::new();
        let mut escaped = false;
        loop {
            match self.cur() {
                Some(c) if is_ascii_id_part(c) => {
                    name.push(char::from(u8::try_from(c).unwrap_or(b'?')));
                    self.pos += 1;
                }
                Some(0x5C) => {
                    let (line, column) = (self.line, self.column());
                    if self.ahead(1) != Some(ascii(b'u')) {
                        return Err(self.error("unexpected character \\"));
                    }
                    self.pos += 2;
                    let cp = self.unicode_escape()?;
                    if cp >= 0x80 {
                        return Err(SyntaxError::refused(
                            "identifier with a \\u escape outside ASCII",
                            5,
                            line,
                            column,
                        ));
                    }
                    let unit = u16::try_from(cp).unwrap_or(0);
                    let allowed = if name.is_empty() {
                        is_ascii_id_start(unit)
                    } else {
                        is_ascii_id_part(unit)
                    };
                    if !allowed {
                        return Err(SyntaxError::new(
                            format!("the escape \\u{{{cp:X}}} is no identifier code point here"),
                            line,
                            column,
                        ));
                    }
                    name.push(char::from(u8::try_from(cp).unwrap_or(b'?')));
                    escaped = true;
                }
                Some(c) if c >= 0x80 && !is_white_space(c) && !is_line_terminator(c) => {
                    return Err(self.refused("identifier with a code point outside ASCII", 5));
                }
                _ => break,
            }
        }
        if escaped {
            return Ok((TokenKind::Name(name), true));
        }
        Ok(match Keyword::from_name(&name) {
            Some(k) => (TokenKind::Keyword(k), false),
            None => (TokenKind::Name(name), false),
        })
    }

    /// Consumes digits of `radix` (`DecimalDigits[+Sep]` and its binary, octal and hexadecimal
    /// forms, `#sec-literals-numeric-literals`): a `NumericLiteralSeparator` stands only between
    /// two digits. Returns how many digits it read.
    fn digits(&mut self, radix: u32) -> Result<usize, SyntaxError> {
        let mut n = 0;
        loop {
            match self.cur() {
                Some(c) if digit_of_radix(c, radix) => {
                    self.pos += 1;
                    n += 1;
                }
                Some(0x5F) => {
                    if n == 0 {
                        return Err(self.error(SEPARATOR_NOT_BETWEEN_DIGITS));
                    }
                    match self.ahead(1) {
                        Some(0x5F) => return Err(self.error(SEPARATOR_DOUBLED)),
                        Some(c) if digit_of_radix(c, radix) => self.pos += 1,
                        _ => return Err(self.error(SEPARATOR_NOT_BETWEEN_DIGITS)),
                    }
                }
                _ => return Ok(n),
            }
        }
    }

    /// A `NumericLiteral` (`#sec-literals-numeric-literals`).
    fn number(&mut self) -> Result<TokenKind, SyntaxError> {
        let start = self.pos;
        let first = self.cur();
        let second = self.ahead(1).map(|c| c | 0x20);
        let mut integer = true;
        let form = if first == Some(ascii(b'0'))
            && matches!(second, Some(0x78) | Some(0x62) | Some(0x6F))
        {
            let (radix, form) = match second {
                Some(0x78) => (16, NumberForm::Hex),
                Some(0x62) => (2, NumberForm::Binary),
                _ => (8, NumberForm::Octal),
            };
            self.pos += 2;
            if self.digits(radix)? == 0 {
                return Err(self.error("missing digits after a numeric literal prefix"));
            }
            form
        } else if first == Some(ascii(b'0')) && self.ahead(1).is_some_and(is_decimal) {
            self.pos += 1;
            let mut octal = true;
            while let Some(c) = self.cur().filter(|&c| is_decimal(c)) {
                octal &= is_octal(c);
                self.pos += 1;
            }
            // `LegacyOctalIntegerLiteral` and `NonOctalDecimalIntegerLiteral` take no separator.
            if self.cur() == Some(0x5F) {
                return Err(self.error(SEPARATOR_AFTER_LEADING_ZERO));
            }
            if octal {
                NumberForm::LegacyOctal
            } else {
                integer = !self.decimal_tail()?;
                NumberForm::NonOctalDecimal
            }
        } else {
            if first == Some(ascii(b'0')) {
                // `DecimalIntegerLiteral :: 0` is one digit, which no separator follows.
                self.pos += 1;
                if self.cur() == Some(0x5F) {
                    return Err(self.error(SEPARATOR_AFTER_LEADING_ZERO));
                }
            } else if first != Some(ascii(b'.')) {
                self.digits(10)?;
            }
            integer = !self.decimal_tail()?;
            NumberForm::Decimal
        };
        if self.cur() == Some(ascii(b'n'))
            && integer
            && !matches!(form, NumberForm::LegacyOctal | NumberForm::NonOctalDecimal)
        {
            return Err(self.refused("BigInt literal", 12));
        }
        if self
            .cur()
            .is_some_and(|c| is_ascii_id_start(c) || is_decimal(c) || c == 0x5C)
        {
            return Err(self.error("identifier start immediately after a numeric literal"));
        }
        let text: String = self
            .src
            .get(start..self.pos)
            .unwrap_or(&[])
            .iter()
            .map(|&u| char::from(u8::try_from(u).unwrap_or(b'?')))
            .collect();
        Ok(TokenKind::Number(NumberLiteral { text, form }))
    }

    /// The fraction and exponent of a decimal literal after its integer digits; whether there was
    /// either.
    fn decimal_tail(&mut self) -> Result<bool, SyntaxError> {
        let mut tail = false;
        if self.cur() == Some(ascii(b'.')) {
            tail = true;
            self.pos += 1;
            self.digits(10)?;
        }
        if matches!(self.cur(), Some(0x65) | Some(0x45)) {
            tail = true;
            self.pos += 1;
            if matches!(self.cur(), Some(0x2B) | Some(0x2D)) {
                self.pos += 1;
            }
            if self.digits(10)? == 0 {
                return Err(self.error("missing digits in the exponent of a numeric literal"));
            }
        }
        Ok(tail)
    }

    /// A `StringLiteral` opened by `quote` (`#sec-literals-string-literals`), its `SV`
    /// (`#sec-string-literals-static-semantics-sv`) as code units.
    fn string(&mut self, quote: u16) -> Result<StringLiteral, SyntaxError> {
        let (line, column) = (self.line, self.column());
        self.pos += 1;
        let mut literal = StringLiteral {
            value: Vec::new(),
            legacy_octal_escape: false,
            non_octal_decimal_escape: false,
        };
        loop {
            let Some(c) = self.cur() else {
                return Err(SyntaxError::new(
                    "unterminated string literal",
                    line,
                    column,
                ));
            };
            if c == quote {
                self.pos += 1;
                return Ok(literal);
            }
            if c == LF || c == CR {
                return Err(SyntaxError::new(
                    "unterminated string literal",
                    line,
                    column,
                ));
            }
            if c == ascii(b'\\') {
                self.pos += 1;
                self.escape(&mut literal)?;
            } else if c == LS || c == PS {
                literal.value.push(c);
                self.line_terminator();
            } else {
                literal.value.push(c);
                self.pos += 1;
            }
        }
    }

    /// The `EscapeSequence` or `LineContinuation` after a `\` in a string literal.
    fn escape(&mut self, literal: &mut StringLiteral) -> Result<(), SyntaxError> {
        let Some(c) = self.cur() else {
            return Err(self.error("unterminated string literal"));
        };
        if is_line_terminator(c) {
            self.line_terminator();
            return Ok(());
        }
        self.pos += 1;
        let value = |u: u8| u16::from(u);
        match c {
            0x62 => literal.value.push(value(0x08)),
            0x66 => literal.value.push(value(0x0C)),
            0x6E => literal.value.push(value(0x0A)),
            0x72 => literal.value.push(value(0x0D)),
            0x74 => literal.value.push(value(0x09)),
            0x76 => literal.value.push(value(0x0B)),
            0x30 if !self.cur().is_some_and(is_decimal) => literal.value.push(0),
            0x30..=0x37 => {
                // `LegacyOctalEscapeSequence`: a `ZeroToThree` takes up to two more octal digits,
                // a `FourToSeven` one; `\0` before 8 or 9 is one too.
                literal.legacy_octal_escape = true;
                let mut v = c - 0x30;
                let more = if c <= 0x33 { 2 } else { 1 };
                for _ in 0..more {
                    match self.cur() {
                        Some(d) if is_octal(d) => {
                            v = v * 8 + (d - 0x30);
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                literal.value.push(v);
            }
            0x38 | 0x39 => {
                literal.non_octal_decimal_escape = true;
                literal.value.push(c);
            }
            0x78 => {
                let hi = self.cur().and_then(hex_value);
                let lo = self.ahead(1).and_then(hex_value);
                match (hi, lo) {
                    (Some(hi), Some(lo)) => {
                        self.pos += 2;
                        literal
                            .value
                            .push(u16::try_from(hi * 16 + lo).unwrap_or(0xFFFD));
                    }
                    _ => return Err(self.error("invalid hexadecimal escape sequence")),
                }
            }
            0x75 => {
                let cp = self.unicode_escape()?;
                push_code_point(&mut literal.value, cp);
            }
            _ => literal.value.push(c),
        }
        Ok(())
    }

    /// The `UnicodeEscapeSequence` after `\u`: `Hex4Digits` or `{ CodePoint }`.
    fn unicode_escape(&mut self) -> Result<u32, SyntaxError> {
        if self.cur() == Some(ascii(b'{')) {
            self.pos += 1;
            let mut cp: u32 = 0;
            let mut n = 0;
            while let Some(d) = self.cur().and_then(hex_value) {
                cp = cp.saturating_mul(16).saturating_add(d);
                n += 1;
                self.pos += 1;
            }
            if n == 0 || cp > 0x10_FFFF || self.cur() != Some(ascii(b'}')) {
                return Err(self.error("invalid Unicode escape sequence"));
            }
            self.pos += 1;
            return Ok(cp);
        }
        let mut cp = 0;
        for i in 0..4 {
            match self.ahead(i).and_then(hex_value) {
                Some(d) => cp = cp * 16 + d,
                None => return Err(self.error("invalid Unicode escape sequence")),
            }
        }
        self.pos += 4;
        Ok(cp)
    }

    /// The token `token`, a `/` or `/=` the parser reads where a `RegularExpressionLiteral` is
    /// allowed, lexed again in the `InputElementRegExp` goal: the literal up to its closing `/`
    /// outside a class, then its flags (`#sec-literals-regular-expression-literals`).
    pub(crate) fn rescan_regexp(&mut self, token: &Token) -> Result<Token, SyntaxError> {
        self.pos = token.start as usize + 1;
        self.line = token.line;
        self.line_start = (token.start as usize + 1).saturating_sub(token.column as usize);
        let body_start = self.pos;
        let mut in_class = false;
        loop {
            let Some(c) = self.cur().filter(|&c| !is_line_terminator(c)) else {
                return Err(SyntaxError::new(
                    "unterminated regular expression literal",
                    token.line,
                    token.column,
                ));
            };
            self.pos += 1;
            match c {
                0x5C => {
                    if self.cur().is_none_or(is_line_terminator) {
                        return Err(SyntaxError::new(
                            "unterminated regular expression literal",
                            token.line,
                            token.column,
                        ));
                    }
                    self.pos += 1;
                }
                0x5B => in_class = true,
                0x5D => in_class = false,
                0x2F if !in_class => break,
                _ => {}
            }
        }
        let body = self
            .src
            .get(body_start..self.pos - 1)
            .unwrap_or(&[])
            .to_vec();
        let flags_start = self.pos;
        while self.cur().is_some_and(is_ascii_id_part) {
            self.pos += 1;
        }
        if let Some(c) = self.cur()
            && c >= 0x80
            && !is_white_space(c)
            && !is_line_terminator(c)
        {
            return Err(self.refused("identifier with a code point outside ASCII", 5));
        }
        let flags = self.src.get(flags_start..self.pos).unwrap_or(&[]).to_vec();
        Ok(Token {
            kind: TokenKind::RegExp(RegExpLiteral { body, flags }),
            start: token.start,
            end: Lexer::offset(self.pos),
            line: token.line,
            column: token.column,
            newline_before: token.newline_before,
            escaped: false,
        })
    }

    /// The `NoSubstitutionTemplate` or `TemplateHead` whose `` ` `` is at the current position
    /// (`#sec-template-literal-lexical-components`).
    fn template(&mut self) -> Result<TokenKind, SyntaxError> {
        let (line, column) = (self.line, self.column());
        self.pos += 1;
        let (part, substitution) = self.template_characters(line, column)?;
        Ok(if substitution {
            TokenKind::TemplateHead(part)
        } else {
            TokenKind::NoSubstitutionTemplate(part)
        })
    }

    /// The token `token`, the `}` that ends a substitution, lexed again in the
    /// `InputElementTemplateTail` goal: a `TemplateMiddle` or a `TemplateTail`
    /// (`TemplateSubstitutionTail`, `#sec-template-literal-lexical-components`). The lexer goes
    /// back to the `}` also when it has lexed the token after it.
    pub(crate) fn rescan_template(&mut self, token: &Token) -> Result<Token, SyntaxError> {
        self.pos = token.start as usize + 1;
        self.line = token.line;
        self.line_start = (token.start as usize + 1).saturating_sub(token.column as usize);
        let (part, substitution) = self.template_characters(token.line, token.column)?;
        Ok(Token {
            kind: if substitution {
                TokenKind::TemplateMiddle(part)
            } else {
                TokenKind::TemplateTail(part)
            },
            start: token.start,
            end: Lexer::offset(self.pos),
            line: token.line,
            column: token.column,
            newline_before: token.newline_before,
            escaped: false,
        })
    }

    /// The `TemplateCharacters` after the `` ` `` or `}` that opens a template token at `line`
    /// and `column`, and its closing `` ` `` (false) or `${` (true), consumed: their TV
    /// (`#sec-static-semantics-tv`: an escape is its SV, a `LineContinuation` nothing, a
    /// `LineTerminatorSequence` its TRV), their TRV (`#sec-static-semantics-trv`) and where the
    /// first NotEscapeSequence is.
    fn template_characters(
        &mut self,
        line: u32,
        column: u32,
    ) -> Result<(Box<TemplatePart>, bool), SyntaxError> {
        let body_start = self.pos;
        let mut cooked = Vec::new();
        let mut not_escape = None;
        let substitution = loop {
            let Some(c) = self.cur() else {
                return Err(SyntaxError::new(UNTERMINATED_TEMPLATE, line, column));
            };
            match c {
                0x60 => break false,
                0x24 if self.ahead(1) == Some(ascii(b'{')) => break true,
                0x5C => {
                    let at = (self.line, self.column());
                    self.pos += 1;
                    if !self.template_escape(&mut cooked) && not_escape.is_none() {
                        not_escape = Some(at);
                    }
                }
                // `LineTerminatorSequence`: the TRV of `<CR>` and of `<CR><LF>` is `<LF>`.
                c if is_line_terminator(c) => {
                    cooked.push(if c == CR { LF } else { c });
                    self.line_terminator();
                }
                _ => {
                    cooked.push(c);
                    self.pos += 1;
                }
            }
        };
        let raw = trv(self.src.get(body_start..self.pos).unwrap_or(&[]));
        self.pos += if substitution { 2 } else { 1 };
        Ok((
            Box::new(TemplatePart {
                string: TemplateString { cooked, raw },
                not_escape,
            }),
            substitution,
        ))
    }

    /// The `TemplateEscapeSequence` or `LineContinuation` after a `\` in a template, its TV
    /// appended to `cooked`; false for a `NotEscapeSequence`, whose TV is undefined. A
    /// `NotEscapeSequence` is never longer than what the characters after the `\` read as
    /// `TemplateCharacter`s, so only the character after the `\` is consumed then: the TRV is
    /// the source text either way.
    fn template_escape(&mut self, cooked: &mut Vec<u16>) -> bool {
        let Some(c) = self.cur() else {
            // The end of the source: the caller finds the template unterminated.
            return true;
        };
        if is_line_terminator(c) {
            self.line_terminator();
            return true;
        }
        self.pos += 1;
        match c {
            0x62 => cooked.push(0x08),
            0x66 => cooked.push(0x0C),
            0x6E => cooked.push(0x0A),
            0x72 => cooked.push(0x0D),
            0x74 => cooked.push(0x09),
            0x76 => cooked.push(0x0B),
            // `0 [lookahead ∉ DecimalDigit]`; before a digit, `0 DecimalDigit` and every other
            // `DecimalDigit` are NotEscapeSequences: no legacy octal escape in a template.
            0x30 if !self.cur().is_some_and(is_decimal) => cooked.push(0),
            0x30..=0x39 => return false,
            0x78 => match (
                self.cur().and_then(hex_value),
                self.ahead(1).and_then(hex_value),
            ) {
                (Some(hi), Some(lo)) => {
                    self.pos += 2;
                    cooked.push(u16::try_from(hi * 16 + lo).unwrap_or(0xFFFD));
                }
                _ => return false,
            },
            0x75 => match self.template_unicode_escape() {
                Some(cp) => push_code_point(cooked, cp),
                None => return false,
            },
            // `NonEscapeCharacter` and the `SingleEscapeCharacter`s that stand for themselves.
            _ => cooked.push(c),
        }
        true
    }

    /// The `UnicodeEscapeSequence` after `\u` in a template, consumed; `None`, with nothing
    /// consumed, where the characters are a `NotEscapeSequence` (no four hexadecimal digits, no
    /// `}`, or a `NotCodePoint`).
    fn template_unicode_escape(&mut self) -> Option<u32> {
        if self.cur() == Some(ascii(b'{')) {
            let mut at = self.pos + 1;
            let mut cp: u32 = 0;
            let mut digits = 0;
            while let Some(d) = self.unit(at).and_then(hex_value) {
                cp = cp.saturating_mul(16).saturating_add(d);
                digits += 1;
                at += 1;
            }
            if digits == 0 || cp > 0x10_FFFF || self.unit(at) != Some(ascii(b'}')) {
                return None;
            }
            self.pos = at + 1;
            return Some(cp);
        }
        let mut cp = 0;
        for i in 0..4 {
            cp = cp * 16 + self.ahead(i).and_then(hex_value)?;
        }
        self.pos += 4;
        Some(cp)
    }

    /// The longest `Punctuator` at the current position, consumed.
    fn punctuator(&mut self, c: u16) -> Option<Punct> {
        let a = self
            .ahead(1)
            .and_then(|u| u8::try_from(u).ok())
            .unwrap_or(0);
        let b = self
            .ahead(2)
            .and_then(|u| u8::try_from(u).ok())
            .unwrap_or(0);
        let d = self
            .ahead(3)
            .and_then(|u| u8::try_from(u).ok())
            .unwrap_or(0);
        let (p, len) = match u8::try_from(c).ok()? {
            b'{' => (Punct::LBrace, 1),
            b'}' => (Punct::RBrace, 1),
            b'(' => (Punct::LParen, 1),
            b')' => (Punct::RParen, 1),
            b'[' => (Punct::LBracket, 1),
            b']' => (Punct::RBracket, 1),
            b';' => (Punct::Semicolon, 1),
            b',' => (Punct::Comma, 1),
            b':' => (Punct::Colon, 1),
            b'~' => (Punct::Tilde, 1),
            b'.' => match (a, b) {
                (b'.', b'.') => (Punct::Ellipsis, 3),
                _ => (Punct::Dot, 1),
            },
            b'<' => match (a, b) {
                (b'<', b'=') => (Punct::ShlEq, 3),
                (b'<', _) => (Punct::Shl, 2),
                (b'=', _) => (Punct::LtEq, 2),
                _ => (Punct::Lt, 1),
            },
            b'>' => match (a, b, d) {
                (b'>', b'>', b'=') => (Punct::UShrEq, 4),
                (b'>', b'>', _) => (Punct::UShr, 3),
                (b'>', b'=', _) => (Punct::ShrEq, 3),
                (b'>', _, _) => (Punct::Shr, 2),
                (b'=', _, _) => (Punct::GtEq, 2),
                _ => (Punct::Gt, 1),
            },
            b'=' => match (a, b) {
                (b'=', b'=') => (Punct::EqEqEq, 3),
                (b'=', _) => (Punct::EqEq, 2),
                (b'>', _) => (Punct::Arrow, 2),
                _ => (Punct::Eq, 1),
            },
            b'!' => match (a, b) {
                (b'=', b'=') => (Punct::NotEqEq, 3),
                (b'=', _) => (Punct::NotEq, 2),
                _ => (Punct::Bang, 1),
            },
            b'+' => match a {
                b'+' => (Punct::PlusPlus, 2),
                b'=' => (Punct::PlusEq, 2),
                _ => (Punct::Plus, 1),
            },
            b'-' => match a {
                b'-' => (Punct::MinusMinus, 2),
                b'=' => (Punct::MinusEq, 2),
                _ => (Punct::Minus, 1),
            },
            b'*' => match (a, b) {
                (b'*', b'=') => (Punct::StarStarEq, 3),
                (b'*', _) => (Punct::StarStar, 2),
                (b'=', _) => (Punct::StarEq, 2),
                _ => (Punct::Star, 1),
            },
            b'%' => match a {
                b'=' => (Punct::PercentEq, 2),
                _ => (Punct::Percent, 1),
            },
            b'&' => match (a, b) {
                (b'&', b'=') => (Punct::AmpAmpEq, 3),
                (b'&', _) => (Punct::AmpAmp, 2),
                (b'=', _) => (Punct::AmpEq, 2),
                _ => (Punct::Amp, 1),
            },
            b'|' => match (a, b) {
                (b'|', b'=') => (Punct::PipePipeEq, 3),
                (b'|', _) => (Punct::PipePipe, 2),
                (b'=', _) => (Punct::PipeEq, 2),
                _ => (Punct::Pipe, 1),
            },
            b'^' => match a {
                b'=' => (Punct::CaretEq, 2),
                _ => (Punct::Caret, 1),
            },
            b'?' => match (a, b) {
                (b'?', b'=') => (Punct::QuestionQuestionEq, 3),
                (b'?', _) => (Punct::QuestionQuestion, 2),
                (b'.', d) if !d.is_ascii_digit() => (Punct::QuestionDot, 2),
                _ => (Punct::Question, 1),
            },
            b'/' => match a {
                b'=' => (Punct::SlashEq, 2),
                _ => (Punct::Slash, 1),
            },
            _ => return None,
        };
        self.pos += len;
        Some(p)
    }
}
