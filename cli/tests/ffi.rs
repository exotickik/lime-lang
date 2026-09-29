use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("lime_ffi_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn extern_abs_from_libc() {
    let src = "program F\n\
        ekstern fungsi abs(x: i32) -> i32\n\
        utama { nilai n: i32 = 0i32 - 42i32 tampilkan(abs(n)) }";
    let (out, code) = run_source("abs", src);
    assert_eq!(out, "42\n");
    assert_eq!(code, 0);
}

#[test]
fn extern_putchar_side_effect() {
    let src = "program F\n\
        ekstern fungsi putchar(c: i32) -> i32\n\
        utama { putchar(65i32) putchar(66i32) putchar(10i32) }";
    let (out, _) = run_source("putchar", src);
    assert_eq!(out, "AB\n");
}

#[test]
fn extern_variadic_printf() {
    let src = "program F\n\
        ekstern fungsi printf(fmt: teks, ...) -> i32\n\
        utama { printf(\"x=%d y=%d\\n\", 3i32, 4i32) }";
    let (out, _) = run_source("printf", src);
    assert_eq!(out, "x=3 y=4\n");
}
