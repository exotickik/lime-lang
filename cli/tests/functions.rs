use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_fn_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

const OPS: &str = "program P\n\
    fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
    fungsi kali(a: angka, b: angka) -> angka { kembalikan a * b }\n\
    fungsi terapkan(f: fungsi(angka, angka) -> angka, x: angka, y: angka) -> angka { kembalikan f(x, y) }\n";

#[test]
fn function_passed_as_argument() {
    let src = format!("{OPS}utama {{ tampilkan(terapkan(tambah, 3, 4)) tampilkan(terapkan(kali, 3, 4)) }}");
    let (out, code) = run_source("arg", &src);
    assert_eq!(out, "7\n12\n");
    assert_eq!(code, 0);
}

#[test]
fn function_stored_and_called() {
    let src = format!("{OPS}utama {{ nilai g = kali tampilkan(g(6, 7)) }}");
    let (out, _) = run_source("var", &src);
    assert_eq!(out, "42\n");
}

#[test]
fn function_returned_from_function() {
    let src = "program P\n\
        fungsi naik(n: angka) -> angka { kembalikan n + 1 }\n\
        fungsi turun(n: angka) -> angka { kembalikan n - 1 }\n\
        fungsi pilih(mode: angka) -> fungsi(angka) -> angka { jika mode == 0 { kembalikan naik } kembalikan turun }\n\
        utama { nilai f = pilih(0) nilai g = pilih(1) tampilkan(f(10)) tampilkan(g(10)) }";
    let (out, _) = run_source("ret", src);
    assert_eq!(out, "11\n9\n");
}

#[test]
fn function_pointer_is_a_plain_pointer_no_alloc() {
    let src = format!("{OPS}utama {{ tampilkan(terapkan(tambah, 1, 1)) }}");
    let path = dir("ir").join("prog.exo");
    std::fs::write(&path, &src).unwrap();
    let out = exotickik().arg("emit-ir").arg(&path).output().unwrap();
    let ir = String::from_utf8_lossy(&out.stdout);
    assert_eq!(ir.matches("call ptr @malloc").count(), 0, "function values are bare pointers");
    assert!(ir.contains("@tambah"), "the function's address is referenced directly");
}
