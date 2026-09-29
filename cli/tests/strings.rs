use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("lime_str_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn content_equality_across_calls() {
    let src = "program S\n\
        fungsi bandingkan(a: teks, b: teks) -> boolean { kembalikan a == b }\n\
        utama {\n\
          jika bandingkan(\"halo\", \"halo\") { tampilkan(\"sama\") } lain { tampilkan(\"beda\") }\n\
          jika bandingkan(\"halo\", \"dunia\") { tampilkan(\"sama\") } lain { tampilkan(\"beda\") }\n\
        }";
    let (out, code) = run_source("eq", src);
    assert_eq!(out, "sama\nbeda\n");
    assert_eq!(code, 0);
}

#[test]
fn inequality_operator() {
    let src = "program S\nutama { jika \"abc\" != \"abd\" { tampilkan(\"tidak sama\") } }";
    let (out, _) = run_source("ne", src);
    assert_eq!(out, "tidak sama\n");
}

#[test]
fn byte_length_ascii() {
    let src = "program S\nutama { tampilkan(panjang(\"Selamat datang\")) }";
    let (out, _) = run_source("len", src);
    assert_eq!(out, "14\n");
}

#[test]
fn byte_length_utf8() {
    let src = "program S\nutama { tampilkan(panjang(\"lémon\")) tampilkan(panjang(\"🍋\")) }";
    let (out, _) = run_source("utf8", src);
    assert_eq!(out, "6\n4\n");
}

#[test]
fn string_returned_from_function() {
    let src = "program S\n\
        fungsi pilih(x: angka) -> teks { jika x > 0 { kembalikan \"positif\" } kembalikan \"non-positif\" }\n\
        utama { tampilkan(pilih(5)) tampilkan(pilih(0)) }";
    let (out, _) = run_source("ret", src);
    assert_eq!(out, "positif\nnon-positif\n");
}
