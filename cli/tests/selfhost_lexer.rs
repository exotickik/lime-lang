use std::path::PathBuf;
use std::process::Command;

use exotickik_diagnostics::DiagnosticEngine;
use exotickik_lexer::{tokenize, Keyword, LitValue, TokenKind};
use exotickik_span::SourceMap;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn compiler(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("compiler").join(rel)
}

// Maps a Rust-lexer TokenKind to the same integer code the self-hosted
// `Token.kode_jenis` assigns, so the two lexers can be compared head-to-head.
fn code_of(k: &TokenKind) -> i64 {
    match k {
        TokenKind::Ident(_) => 0,
        TokenKind::Keyword(_) => 1,
        TokenKind::Literal(LitValue::Int { .. }) => 2,
        TokenKind::Literal(LitValue::Float { .. }) => 3,
        TokenKind::Literal(LitValue::Str(_)) => 4,
        TokenKind::Literal(LitValue::Char(_)) => 5,
        TokenKind::LParen => 6,
        TokenKind::RParen => 7,
        TokenKind::LBrace => 8,
        TokenKind::RBrace => 9,
        TokenKind::LBracket => 10,
        TokenKind::RBracket => 11,
        TokenKind::Comma => 12,
        TokenKind::Colon => 13,
        TokenKind::Semicolon => 14,
        TokenKind::Dot => 15,
        TokenKind::Arrow => 16,
        TokenKind::FatArrow => 17,
        TokenKind::Eq => 18,
        TokenKind::EqEq => 19,
        TokenKind::Ne => 20,
        TokenKind::Lt => 21,
        TokenKind::Le => 22,
        TokenKind::Gt => 23,
        TokenKind::Ge => 24,
        TokenKind::Plus => 25,
        TokenKind::Minus => 26,
        TokenKind::Star => 27,
        TokenKind::Slash => 28,
        TokenKind::Percent => 29,
        TokenKind::AmpAmp => 30,
        TokenKind::Amp => 31,
        TokenKind::PipePipe => 32,
        TokenKind::Bang => 33,
        TokenKind::Eof => 34,
    }
}

fn rust_codes(src: &str) -> Vec<i64> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.exo", src);
    let mut diags = DiagnosticEngine::new();
    tokenize(f, src, &mut diags).iter().map(|t| code_of(&t.kind)).collect()
}

// Keyword code mirroring the order in Token.exo (kode_kata_kunci).
fn kw_code(k: Keyword) -> i64 {
    use Keyword::*;
    match k {
        Program => 0, Modul => 1, Utama => 2, Fungsi => 3, Kembalikan => 4, Nilai => 5,
        Ubah => 6, Jika => 7, Lain => 8, Selama => 9, Untuk => 10, Dari => 11, Sampai => 12,
        Hingga => 13, Pada => 14, Berhenti => 15, Lanjut => 16, Benar => 17, Salah => 18,
        Tipe => 19, Enum => 20, Sifat => 21, Implementasi => 22, Cocok => 23, Coba => 24,
        Gunakan => 25, Impor => 26, Ekstern => 27, Publik => 28, Dan => 29, Atau => 30,
        Bukan => 31, Sebagai => 32,
    }
}

#[test]
fn self_hosted_lexer_passes_its_own_suite() {
    let out = exotickik().arg("test").arg(compiler("uji_lekser.exo")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("7 lulus, 0 gagal"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn rust_lexer_agrees_with_representative_token_streams() {
    // These sequences are exactly what compiler/uji_lekser.exo asserts the
    // self-hosted lexer produces; here we confirm the Rust oracle produces the
    // same, establishing head-to-head agreement on each input.
    assert_eq!(rust_codes("fungsi tambah nilai x"), vec![1, 0, 1, 0, 34]);
    assert_eq!(
        rust_codes("-> => == != <= >= && || + - * / % = < >"),
        vec![16, 17, 19, 20, 22, 24, 30, 32, 25, 26, 27, 28, 29, 18, 21, 23, 34]
    );
    assert_eq!(rust_codes("42 0xFF 0b1010 1_000 10i32"), vec![2, 2, 2, 2, 2, 34]);
    assert_eq!(rust_codes("3.14 2.5e-3"), vec![3, 3, 34]);
    assert_eq!(rust_codes("\"halo\" 'a'"), vec![4, 5, 34]);
    assert_eq!(
        rust_codes("fungsi f(a: angka) -> angka { kembalikan a + 1 }"),
        vec![1, 0, 6, 0, 13, 0, 7, 16, 0, 8, 1, 0, 25, 2, 9, 34]
    );
}

#[test]
fn keyword_codes_align() {
    // Confirms the keyword numbering shared by Token.kode_kata_kunci and the
    // self-hosted lexer matches the Rust Keyword enum.
    assert_eq!(kw_code(Keyword::Fungsi), 3);
    assert_eq!(kw_code(Keyword::Nilai), 5);
    assert_eq!(kw_code(Keyword::Sebagai), 32);
}
