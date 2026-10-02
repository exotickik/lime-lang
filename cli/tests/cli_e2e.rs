use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn workdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("exotickik_e2e_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_program(dir: &PathBuf, src: &str) -> PathBuf {
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    path
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = workdir(tag);
    let path = write_program(&dir, src);
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check_source(tag: &str, src: &str) -> (String, i32) {
    let dir = workdir(tag);
    let path = write_program(&dir, src);
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn hello_world_prints() {
    let (out, code) = run_source("hello", "program H\nutama { tampilkan(\"Halo dunia!\") }");
    assert_eq!(out, "Halo dunia!\n");
    assert_eq!(code, 0);
}

#[test]
fn arithmetic_precedence_runtime() {
    let (out, _) = run_source(
        "arith",
        "program A\nutama { tampilkan(1 + 2 * 3 - 4) }",
    );
    assert_eq!(out, "3\n");
}

#[test]
fn recursion_fibonacci() {
    let src = "program F\n\
        fungsi fib(n: angka) -> angka { jika n < 2 { kembalikan n } kembalikan fib(n-1)+fib(n-2) }\n\
        utama { tampilkan(fib(10)) }";
    let (out, _) = run_source("fib", src);
    assert_eq!(out, "55\n");
}

#[test]
fn while_loop_and_mutation() {
    let src = "program W\nutama { ubah n = 1 selama n < 100 { n = n * 2 } tampilkan(n) }";
    let (out, _) = run_source("while", src);
    assert_eq!(out, "128\n");
}

#[test]
fn for_inclusive_sum() {
    let src = "program S\nutama { ubah s = 0 untuk i dari 1 hingga 100 { s = s + i } tampilkan(s) }";
    let (out, _) = run_source("sum", src);
    assert_eq!(out, "5050\n");
}

#[test]
fn boolean_and_short_circuit() {
    let src = "program B\n\
        utama { nilai a = benar nilai b = salah tampilkan(a dan b) tampilkan(a atau b) }";
    let (out, _) = run_source("bool", src);
    assert_eq!(out, "salah\nbenar\n");
}

#[test]
fn conditional_branches() {
    let src = "program C\n\
        utama { nilai x = 7 jika x > 10 { tampilkan(\"besar\") } lain jika x > 5 { tampilkan(\"sedang\") } lain { tampilkan(\"kecil\") } }";
    let (out, _) = run_source("cond", src);
    assert_eq!(out, "sedang\n");
}

#[test]
fn typed_i32_arithmetic() {
    let src = "program T\nutama { nilai a: i32 = 1000i32 nilai b: i32 = 337i32 tampilkan(a + b) }";
    let (out, _) = run_source("i32", src);
    assert_eq!(out, "1337\n");
}

#[test]
fn exit_code_from_main() {
    let (_out, code) = run_source("exit", "program E\nutama { kembalikan 7 }");
    assert_eq!(code, 7);
}

#[test]
fn invalid_program_fails_check() {
    let (stderr, code) = check_source("bad", "program X\nutama { tampilkan(tidakada) }");
    assert_eq!(code, 1);
    assert!(stderr.contains("E0201"), "stderr: {stderr}");
}

#[test]
fn type_mismatch_reported() {
    let (stderr, code) = check_source(
        "mismatch",
        "program X\nutama { nilai x: i32 = 1i32 nilai y: i64 = 2i64 tampilkan(x + y) }",
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("E0210"), "stderr: {stderr}");
}
