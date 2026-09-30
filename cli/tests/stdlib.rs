use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_std_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check_stderr(tag: &str, src: &str) -> (i32, String) {
    let path = dir(tag).join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("check").arg(&path).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn inti_helpers() {
    let src = "program P\ngunakan Inti\n\
        utama { tampilkan(maks(3, 9)) tampilkan(min(3, 9)) tampilkan(mutlak(0 - 7)) \
        tampilkan(batas(15, 0, 10)) tampilkan(tanda(0 - 4)) }";
    let (out, code) = run_source("inti", src);
    assert_eq!(out, "9\n3\n7\n10\n-1\n");
    assert_eq!(code, 0);
}

#[test]
fn matematika_helpers() {
    let src = "program P\ngunakan Matematika\n\
        utama { tampilkan(pangkat(2, 10)) tampilkan(fpb(48, 36)) tampilkan(kpk(4, 6)) \
        tampilkan(apakah_prima(17)) tampilkan(apakah_prima(18)) }";
    let (out, _) = run_source("math", src);
    assert_eq!(out, "1024\n12\n12\nbenar\nsalah\n");
}

#[test]
fn matematika_faktorial_uses_hasil() {
    let src = "program P\ngunakan Matematika\n\
        utama { nilai a = faktorial(5) nilai b = faktorial(0 - 1) \
        tampilkan(nilai_atau(a, 0)) tampilkan(gagal_atau(b, \"?\")) }";
    let (out, _) = run_source("fakt", src);
    assert_eq!(out, "120\nfaktorial butuh angka tak negatif\n");
}

#[test]
fn two_std_modules_together() {
    let src = "program P\ngunakan Inti\ngunakan Matematika\n\
        utama { tampilkan(maks(pangkat(2, 3), fpb(12, 18))) }";
    let (out, _) = run_source("both", src);
    assert_eq!(out, "8\n");
}

#[test]
fn unknown_std_module_is_error() {
    let src = "program P\ngunakan Fiksi\nutama { }";
    let (code, err) = check_stderr("unknown", src);
    assert_eq!(code, 1);
    assert!(err.contains("tidak dikenal"), "got: {err}");
}

#[test]
fn std_function_needs_gunakan() {
    let src = "program P\nutama { tampilkan(maks(1, 2)) }";
    let (code, _) = check_stderr("noguna", src);
    assert_eq!(code, 1);
}
