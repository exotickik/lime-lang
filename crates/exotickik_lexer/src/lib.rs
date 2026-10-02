pub mod token;

pub use token::{Keyword, LitValue, Token, TokenKind};

use exotickik_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use exotickik_span::{FileId, Span};

pub struct Lexer<'a> {
    file: FileId,
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    diags: &'a mut DiagnosticEngine,
}

impl<'a> Lexer<'a> {
    pub fn new(file: FileId, src: &'a str, diags: &'a mut DiagnosticEngine) -> Lexer<'a> {
        Lexer { file, src, bytes: src.as_bytes(), pos: 0, diags }
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            let start = self.pos;
            if self.pos >= self.bytes.len() {
                tokens.push(Token { kind: TokenKind::Eof, span: self.span(start, start) });
                break;
            }
            if let Some(tok) = self.next_token(start) {
                tokens.push(tok);
            }
        }
        tokens
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file, start as u32, end as u32)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_whitespace() => {
                    self.pos += 1;
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    self.pos += 2;
                    while let Some(b) = self.peek() {
                        if b == b'\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                Some(b'/') if self.peek2() == Some(b'*') => {
                    let start = self.pos;
                    self.pos += 2;
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek() {
                            Some(b'/') if self.peek2() == Some(b'*') => {
                                self.pos += 2;
                                depth += 1;
                            }
                            Some(b'*') if self.peek2() == Some(b'/') => {
                                self.pos += 2;
                                depth -= 1;
                            }
                            Some(_) => self.pos += 1,
                            None => {
                                self.diags.emit(
                                    Diagnostic::error(
                                        codes::E_UNTERMINATED_COMMENT,
                                        "komentar blok tidak ditutup",
                                    )
                                    .primary(self.span(start, self.pos), "dimulai di sini")
                                    .help("tambahkan '*/' untuk menutup komentar"),
                                );
                                return;
                            }
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn next_token(&mut self, start: usize) -> Option<Token> {
        let b = self.peek().unwrap();

        if b == b'_' || (b as char).is_ascii_alphabetic() || b >= 0x80 {
            return Some(self.lex_ident_or_keyword(start));
        }
        if b.is_ascii_digit() {
            return self.lex_number(start);
        }
        match b {
            b'"' => return self.lex_string(start),
            b'\'' => return self.lex_char(start),
            _ => {}
        }

        let kind = self.lex_operator(start);
        match kind {
            Some(k) => Some(Token { kind: k, span: self.span(start, self.pos) }),
            None => {

                let ch_len = utf8_len(b);
                self.pos += ch_len;
                let sp = self.span(start, self.pos);
                let ch = self.src[start..self.pos].chars().next().unwrap_or('?');
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNKNOWN_CHAR,
                        format!("karakter tidak dikenal '{ch}'"),
                    )
                    .primary(sp, "tidak dikenali di sini"),
                );
                None
            }
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> Token {
        while let Some(b) = self.peek() {
            if b == b'_' || (b as char).is_ascii_alphanumeric() || b >= 0x80 {
                self.pos += utf8_len(b);
            } else {
                break;
            }
        }
        let text = &self.src[start..self.pos];
        let kind = match Keyword::from_str(text) {
            Some(kw) => TokenKind::Keyword(kw),
            None => TokenKind::Ident(text.to_string()),
        };
        Token { kind, span: self.span(start, self.pos) }
    }

    fn lex_number(&mut self, start: usize) -> Option<Token> {

        if self.peek() == Some(b'0') {
            if let Some(radix_byte) = self.peek2() {
                let radix = match radix_byte {
                    b'x' | b'X' => Some(16),
                    b'b' | b'B' => Some(2),
                    b'o' | b'O' => Some(8),
                    _ => None,
                };
                if let Some(radix) = radix {
                    self.pos += 2;
                    let digits_start = self.pos;
                    let mut digits = String::new();
                    while let Some(b) = self.peek() {
                        if b == b'_' {
                            self.pos += 1;
                        } else if (b as char).is_digit(radix) {
                            digits.push(b as char);
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    let suffix = self.lex_num_suffix();
                    if digits.is_empty() {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_BAD_NUMBER,
                                "literal angka tanpa digit",
                            )
                            .primary(self.span(start, self.pos), "digit hilang"),
                        );
                        return None;
                    }
                    let value = u128::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                        self.diags.emit(
                            Diagnostic::error(codes::E_BAD_NUMBER, "literal angka terlalu besar")
                                .primary(self.span(start, self.pos), "melebihi 128-bit"),
                        );
                        0
                    });
                    let _ = digits_start;
                    return Some(Token {
                        kind: TokenKind::Literal(LitValue::Int { value, suffix }),
                        span: self.span(start, self.pos),
                    });
                }
            }
        }

        let mut is_float = false;
        let mut mantissa = String::new();
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                mantissa.push(b as char);
                self.pos += 1;
            } else if b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.peek() == Some(b'.') && self.peek2().map_or(false, |c| c.is_ascii_digit()) {
            is_float = true;
            mantissa.push('.');
            self.pos += 1;
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() {
                    mantissa.push(b as char);
                    self.pos += 1;
                } else if b == b'_' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            mantissa.push('e');
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                mantissa.push(self.peek().unwrap() as char);
                self.pos += 1;
            }
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() {
                    mantissa.push(b as char);
                    self.pos += 1;
                } else if b == b'_' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        let suffix = self.lex_num_suffix();
        let sp = self.span(start, self.pos);
        if is_float {
            match mantissa.parse::<f64>() {
                Ok(value) => Some(Token {
                    kind: TokenKind::Literal(LitValue::Float { value, suffix }),
                    span: sp,
                }),
                Err(_) => {
                    self.diags.emit(
                        Diagnostic::error(codes::E_BAD_NUMBER, "literal pecahan tidak valid")
                            .primary(sp, "tidak bisa diurai"),
                    );
                    None
                }
            }
        } else {
            match mantissa.parse::<u128>() {
                Ok(value) => Some(Token {
                    kind: TokenKind::Literal(LitValue::Int { value, suffix }),
                    span: sp,
                }),
                Err(_) => {
                    self.diags.emit(
                        Diagnostic::error(codes::E_BAD_NUMBER, "literal bilangan bulat terlalu besar")
                            .primary(sp, "melebihi 128-bit"),
                    );
                    None
                }
            }
        }
    }

    fn lex_num_suffix(&mut self) -> Option<String> {
        let start = self.pos;
        if !matches!(self.peek(), Some(b'i' | b'u' | b'f')) {
            return None;
        }

        let mut end = self.pos;
        while let Some(b) = self.bytes.get(end) {
            if b.is_ascii_alphanumeric() {
                end += 1;
            } else {
                break;
            }
        }
        let candidate = &self.src[start..end];
        const SUFFIXES: &[&str] =
            &["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64"];
        if SUFFIXES.contains(&candidate) {
            self.pos = end;
            Some(candidate.to_string())
        } else {
            None
        }
    }

    fn lex_string(&mut self, start: usize) -> Option<Token> {
        self.pos += 1;
        let mut value = String::new();
        loop {
            match self.bump() {
                None | Some(b'\n') => {
                    self.diags.emit(
                        Diagnostic::error(codes::E_UNTERMINATED_STRING, "teks tidak ditutup")
                            .primary(self.span(start, self.pos), "diawali di sini")
                            .help("tutup teks dengan '\"'"),
                    );
                    return None;
                }
                Some(b'"') => break,
                Some(b'\\') => {
                    if let Some(c) = self.lex_escape(start) {
                        value.push(c);
                    }
                }
                Some(b) if b < 0x80 => value.push(b as char),
                Some(b) => {

                    self.pos -= 1;
                    let len = utf8_len(b);
                    let ch = self.src[self.pos..self.pos + len].chars().next().unwrap();
                    value.push(ch);
                    self.pos += len;
                }
            }
        }
        Some(Token {
            kind: TokenKind::Literal(LitValue::Str(value)),
            span: self.span(start, self.pos),
        })
    }

    fn lex_char(&mut self, start: usize) -> Option<Token> {
        self.pos += 1;
        let ch = match self.bump() {
            Some(b'\\') => self.lex_escape(start),
            Some(b'\'') => {
                self.diags.emit(
                    Diagnostic::error(codes::E_BAD_ESCAPE, "literal karakter kosong")
                        .primary(self.span(start, self.pos), "tidak ada karakter"),
                );
                None
            }
            Some(b) if b < 0x80 => Some(b as char),
            Some(b) => {
                self.pos -= 1;
                let len = utf8_len(b);
                let c = self.src[self.pos..self.pos + len].chars().next().unwrap();
                self.pos += len;
                Some(c)
            }
            None => None,
        };

        if self.peek() == Some(b'\'') {
            self.pos += 1;
        } else {
            self.diags.emit(
                Diagnostic::error(codes::E_BAD_ESCAPE, "literal karakter tidak ditutup")
                    .primary(self.span(start, self.pos), "harus diakhiri dengan \"'\""),
            );
        }
        ch.map(|c| Token {
            kind: TokenKind::Literal(LitValue::Char(c)),
            span: self.span(start, self.pos),
        })
    }

    fn lex_escape(&mut self, start: usize) -> Option<char> {
        match self.bump() {
            Some(b'n') => Some('\n'),
            Some(b'r') => Some('\r'),
            Some(b't') => Some('\t'),
            Some(b'0') => Some('\0'),
            Some(b'\\') => Some('\\'),
            Some(b'"') => Some('"'),
            Some(b'\'') => Some('\''),
            Some(b'u') => self.lex_unicode_escape(start),
            other => {
                let shown = other.map(|b| b as char).unwrap_or(' ');
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_BAD_ESCAPE,
                        format!("escape tidak dikenal '\\{shown}'"),
                    )
                    .primary(self.span(start, self.pos), "escape tidak valid"),
                );
                None
            }
        }
    }

    fn lex_unicode_escape(&mut self, start: usize) -> Option<char> {
        if self.peek() != Some(b'{') {
            self.diags.emit(
                Diagnostic::error(codes::E_BAD_ESCAPE, "escape unicode butuh '{'")
                    .primary(self.span(start, self.pos), "format: \\u{XXXX}"),
            );
            return None;
        }
        self.pos += 1;
        let mut hex = String::new();
        while let Some(b) = self.peek() {
            if b == b'}' {
                break;
            }
            if (b as char).is_ascii_hexdigit() {
                hex.push(b as char);
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.peek() == Some(b'}') {
            self.pos += 1;
        }
        match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
            Some(c) => Some(c),
            None => {
                self.diags.emit(
                    Diagnostic::error(codes::E_BAD_ESCAPE, "kode unicode tidak valid")
                        .primary(self.span(start, self.pos), "bukan skalar unicode yang sah"),
                );
                None
            }
        }
    }

    fn lex_operator(&mut self, _start: usize) -> Option<TokenKind> {
        let b = self.peek()?;
        let two = self.peek2();

        let two_char = match (b, two) {
            (b'-', Some(b'>')) => Some(TokenKind::Arrow),
            (b'=', Some(b'>')) => Some(TokenKind::FatArrow),
            (b'=', Some(b'=')) => Some(TokenKind::EqEq),
            (b'!', Some(b'=')) => Some(TokenKind::Ne),
            (b'<', Some(b'=')) => Some(TokenKind::Le),
            (b'>', Some(b'=')) => Some(TokenKind::Ge),
            (b'&', Some(b'&')) => Some(TokenKind::AmpAmp),
            (b'|', Some(b'|')) => Some(TokenKind::PipePipe),
            _ => None,
        };
        if let Some(k) = two_char {
            self.pos += 2;
            return Some(k);
        }
        let one = match b {
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,
            b',' => TokenKind::Comma,
            b':' => TokenKind::Colon,
            b';' => TokenKind::Semicolon,
            b'.' => TokenKind::Dot,
            b'=' => TokenKind::Eq,
            b'<' => TokenKind::Lt,
            b'>' => TokenKind::Gt,
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'!' => TokenKind::Bang,
            b'&' => TokenKind::Amp,
            _ => return None,
        };
        self.pos += 1;
        Some(one)
    }
}

fn utf8_len(lead: u8) -> usize {
    match lead {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

pub fn tokenize(file: FileId, src: &str, diags: &mut DiagnosticEngine) -> Vec<Token> {
    Lexer::new(file, src, diags).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use exotickik_diagnostics::DiagnosticEngine;
    use exotickik_span::SourceMap;

    fn lex(src: &str) -> (Vec<TokenKind>, DiagnosticEngine) {
        let mut sm = SourceMap::new();
        let f = sm.add("t.exo", src);
        let mut diags = DiagnosticEngine::new();
        let toks = tokenize(f, src, &mut diags);
        (toks.into_iter().map(|t| t.kind).collect(), diags)
    }

    #[test]
    fn keywords_and_idents() {
        let (kinds, d) = lex("fungsi tambah nilai x");
        assert!(!d.has_errors());
        assert_eq!(kinds[0], TokenKind::Keyword(Keyword::Fungsi));
        assert_eq!(kinds[1], TokenKind::Ident("tambah".into()));
        assert_eq!(kinds[2], TokenKind::Keyword(Keyword::Nilai));
        assert_eq!(kinds[3], TokenKind::Ident("x".into()));
        assert_eq!(kinds[4], TokenKind::Eof);
    }

    #[test]
    fn operators() {
        let (kinds, d) = lex("-> => == != <= >= && || + - * / % = < >");
        assert!(!d.has_errors());
        use TokenKind::*;
        let expected = [
            Arrow, FatArrow, EqEq, Ne, Le, Ge, AmpAmp, PipePipe, Plus, Minus, Star, Slash, Percent,
            Eq, Lt, Gt, Eof,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn numbers_int_float_radix_suffix() {
        let (kinds, d) = lex("42 3.14 0xFF 0b1010 1_000 10i32 2.5e-3 255u8");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert_eq!(kinds[0], TokenKind::Literal(LitValue::Int { value: 42, suffix: None }));
        assert!(matches!(kinds[1], TokenKind::Literal(LitValue::Float { .. })));
        assert_eq!(kinds[2], TokenKind::Literal(LitValue::Int { value: 255, suffix: None }));
        assert_eq!(kinds[3], TokenKind::Literal(LitValue::Int { value: 10, suffix: None }));
        assert_eq!(kinds[4], TokenKind::Literal(LitValue::Int { value: 1000, suffix: None }));
        assert_eq!(
            kinds[5],
            TokenKind::Literal(LitValue::Int { value: 10, suffix: Some("i32".into()) })
        );
        assert!(matches!(kinds[6], TokenKind::Literal(LitValue::Float { .. })));
        assert_eq!(
            kinds[7],
            TokenKind::Literal(LitValue::Int { value: 255, suffix: Some("u8".into()) })
        );
    }

    #[test]
    fn strings_and_escapes() {
        let (kinds, d) = lex(r#""halo\n" "emoji 🍋" 'a' '\u{1F34B}'"#);
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert_eq!(kinds[0], TokenKind::Literal(LitValue::Str("halo\n".into())));
        assert_eq!(kinds[1], TokenKind::Literal(LitValue::Str("emoji 🍋".into())));
        assert_eq!(kinds[2], TokenKind::Literal(LitValue::Char('a')));
        assert_eq!(kinds[3], TokenKind::Literal(LitValue::Char('🍋')));
    }

    #[test]
    fn nested_block_comment() {
        let (kinds, d) = lex("nilai /* a /* b */ c */ x");
        assert!(!d.has_errors());
        assert_eq!(kinds[0], TokenKind::Keyword(Keyword::Nilai));
        assert_eq!(kinds[1], TokenKind::Ident("x".into()));
    }

    #[test]
    fn unterminated_string_reports() {
        let (_k, d) = lex("\"tak selesai");
        assert!(d.has_errors());
    }

    #[test]
    fn unknown_char_reports_but_recovers() {
        let (kinds, d) = lex("nilai @ x");
        assert!(d.has_errors());

        assert_eq!(kinds[0], TokenKind::Keyword(Keyword::Nilai));
        assert_eq!(kinds[1], TokenKind::Ident("x".into()));
    }
}
