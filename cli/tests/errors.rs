use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_err_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check_stderr(tag: &str, src: &str) -> (i32, String) {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn emit_ir(tag: &str, src: &str) -> String {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("emit-ir").arg(&path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const BAGI: &str = "program E\n\
    fungsi bagi(a: angka, b: angka) -> Hasil<angka, teks> {\n\
        jika b == 0 { kembalikan gagal(\"pembagian dengan nol\") }\n\
        kembalikan sukses(a / b)\n\
    }\n";

#[test]
fn success_value_inspected() {
    let src = format!(
        "{BAGI}utama {{ nilai r = bagi(20, 4) tampilkan(berhasil(r)) tampilkan(nilai_atau(r, 0)) }}"
    );
    let (out, code) = run_source("ok", &src);
    assert_eq!(out, "benar\n5\n");
    assert_eq!(code, 0);
}

#[test]
fn failure_value_inspected() {
    let src = format!(
        "{BAGI}utama {{ nilai r = bagi(20, 0) tampilkan(berhasil(r)) \
         tampilkan(nilai_atau(r, -1)) tampilkan(gagal_atau(r, \"?\")) }}"
    );
    let (out, _) = run_source("fail", &src);
    assert_eq!(out, "salah\n-1\npembagian dengan nol\n");
}

#[test]
fn propagation_returns_early_on_failure() {
    let src = format!(
        "{BAGI}fungsi rantai(a: angka, b: angka, c: angka) -> Hasil<angka, teks> {{ \
         nilai x = coba bagi(a, b) nilai y = coba bagi(x, c) kembalikan sukses(x + y) }}\n\
         utama {{ \
           nilai ok = rantai(100, 5, 2) tampilkan(nilai_atau(ok, 0)) \
           nilai bad = rantai(100, 0, 2) tampilkan(gagal_atau(bad, \"?\")) \
           nilai bad2 = rantai(100, 5, 0) tampilkan(nilai_atau(bad2, -1)) }}"
    );
    let (out, _) = run_source("prop", &src);
    assert_eq!(out, "30\npembagian dengan nol\n-1\n");
}

#[test]
fn hasil_is_zero_cost_no_allocation() {
    let src = format!("{BAGI}utama {{ nilai r = bagi(9, 3) tampilkan(nilai_atau(r, 0)) }}");
    let ir = emit_ir("zerocost", &src);
    assert_eq!(ir.matches("call ptr @malloc").count(), 0, "Hasil is a value, no heap");
    assert!(ir.contains("{ i32,"), "Hasil lowers to a tagged aggregate");
}

#[test]
fn generic_hasil_runs() {
    let src = "program E\n\
        fungsi bungkus<T>(x: T) -> Hasil<T, teks> { kembalikan sukses(x) }\n\
        utama { nilai a = bungkus(42) nilai b = bungkus(7i32) \
          tampilkan(nilai_atau(a, 0)) tampilkan(nilai_atau(b, 0i32)) }";
    let (out, _) = run_source("generic", src);
    assert_eq!(out, "42\n7\n");
}

#[test]
fn sukses_without_context_is_error() {
    let src = "program E\nutama { nilai r = sukses(5) tampilkan(berhasil(r)) }";
    let (code, err) = check_stderr("noctx", src);
    assert_eq!(code, 1);
    assert!(err.contains("E02B0"), "got: {err}");
}

#[test]
fn coba_outside_hasil_is_error() {
    let src = format!("{BAGI}utama {{ nilai y = coba bagi(1, 1) }}");
    let (code, err) = check_stderr("badcoba", &src);
    assert_eq!(code, 1);
    assert!(err.contains("E02B2"), "got: {err}");
}

#[test]
fn owned_payload_is_error() {
    let src = "program E\nfungsi f() -> Hasil<kotak<angka>, teks> { kembalikan gagal(\"x\") }\nutama { }";
    let (code, err) = check_stderr("owned", src);
    assert_eq!(code, 1);
    assert!(err.contains("E02B3"), "got: {err}");
}
