//! Token definitions for Lime.

use lime_span::Span;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keyword {
    Program,
    Utama,
    Fungsi,
    Kembalikan,
    Nilai,
    Ubah,
    Jika,
    Lain,
    Selama,
    Untuk,
    Dari,
    Sampai,
    Hingga,
    Henti,
    Lanjut,
    Benar,
    Salah,
    Tipe,
    Enum,
    Cocok,
    Gunakan,
    Impor,
    Ekstern,
    Publik,
    Dan,
    Atau,
    Bukan,
}

impl Keyword {
    pub fn from_str(s: &str) -> Option<Keyword> {
        use Keyword::*;
        Some(match s {
            "program" => Program,
            "utama" => Utama,
            "fungsi" => Fungsi,
            "kembalikan" => Kembalikan,
            "nilai" => Nilai,
            "ubah" => Ubah,
            "jika" => Jika,
            "lain" => Lain,
            "selama" => Selama,
            "untuk" => Untuk,
            "dari" => Dari,
            "sampai" => Sampai,
            "hingga" => Hingga,
            "henti" => Henti,
            "lanjut" => Lanjut,
            "benar" => Benar,
            "salah" => Salah,
            "tipe" => Tipe,
            "enum" => Enum,
            "cocok" => Cocok,
            "gunakan" => Gunakan,
            "impor" => Impor,
            "ekstern" => Ekstern,
            "publik" => Publik,
            "dan" => Dan,
            "atau" => Atau,
            "bukan" => Bukan,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        use Keyword::*;
        match self {
            Program => "program",
            Utama => "utama",
            Fungsi => "fungsi",
            Kembalikan => "kembalikan",
            Nilai => "nilai",
            Ubah => "ubah",
            Jika => "jika",
            Lain => "lain",
            Selama => "selama",
            Untuk => "untuk",
            Dari => "dari",
            Sampai => "sampai",
            Hingga => "hingga",
            Henti => "henti",
            Lanjut => "lanjut",
            Benar => "benar",
            Salah => "salah",
            Tipe => "tipe",
            Enum => "enum",
            Cocok => "cocok",
            Gunakan => "gunakan",
            Impor => "impor",
            Ekstern => "ekstern",
            Publik => "publik",
            Dan => "dan",
            Atau => "atau",
            Bukan => "bukan",
        }
    }
}

/// A decoded literal value carried by the token, so later stages don't re-parse.
#[derive(Clone, PartialEq, Debug)]
pub enum LitValue {
    /// Raw integer magnitude plus an optional explicit type suffix (e.g. "i32").
    Int { value: u128, suffix: Option<String> },
    Float { value: f64, suffix: Option<String> },
    /// Decoded string contents (escapes resolved).
    Str(String),
    Char(char),
}

#[derive(Clone, PartialEq, Debug)]
pub enum TokenKind {
    // Identifiers & keywords
    Ident(String),
    Keyword(Keyword),

    // Literals
    Literal(LitValue),

    // Punctuation / operators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Arrow, // ->
    Eq,    // =
    EqEq,  // ==
    Ne,    // !=
    Lt,    // <
    Le,    // <=
    Gt,    // >
    Ge,    // >=
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AmpAmp, // &&
    PipePipe, // ||
    Bang,   // !

    /// End of file.
    Eof,
}

impl TokenKind {
    /// A short human description used in "expected X, found Y" diagnostics.
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(_) => "pengenal".to_string(),
            TokenKind::Keyword(k) => format!("kata kunci '{}'", k.as_str()),
            TokenKind::Literal(_) => "literal".to_string(),
            TokenKind::LParen => "'('".to_string(),
            TokenKind::RParen => "')'".to_string(),
            TokenKind::LBrace => "'{'".to_string(),
            TokenKind::RBrace => "'}'".to_string(),
            TokenKind::LBracket => "'['".to_string(),
            TokenKind::RBracket => "']'".to_string(),
            TokenKind::Comma => "','".to_string(),
            TokenKind::Colon => "':'".to_string(),
            TokenKind::Semicolon => "';'".to_string(),
            TokenKind::Dot => "'.'".to_string(),
            TokenKind::Arrow => "'->'".to_string(),
            TokenKind::Eq => "'='".to_string(),
            TokenKind::EqEq => "'=='".to_string(),
            TokenKind::Ne => "'!='".to_string(),
            TokenKind::Lt => "'<'".to_string(),
            TokenKind::Le => "'<='".to_string(),
            TokenKind::Gt => "'>'".to_string(),
            TokenKind::Ge => "'>='".to_string(),
            TokenKind::Plus => "'+'".to_string(),
            TokenKind::Minus => "'-'".to_string(),
            TokenKind::Star => "'*'".to_string(),
            TokenKind::Slash => "'/'".to_string(),
            TokenKind::Percent => "'%'".to_string(),
            TokenKind::AmpAmp => "'&&'".to_string(),
            TokenKind::PipePipe => "'||'".to_string(),
            TokenKind::Bang => "'!'".to_string(),
            TokenKind::Eof => "akhir berkas".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
