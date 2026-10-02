use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("exotickik_reg_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn reg_printf_return_discarded_no_crash() {
    let out = run_source("printf_discard", "program R\nutama { tampilkan(\"a\") tampilkan(\"b\") }");
    assert_eq!(out, "a\nb\n");
}

#[test]
fn reg_i32_print_uses_correct_cast() {
    let out = run_source("i32cast", "program R\nutama { nilai n: i32 = 42i32 tampilkan(n) }");
    assert_eq!(out, "42\n");
}

#[test]
fn reg_i64_print_full_width() {
    let out = run_source("i64wide", "program R\nutama { nilai n: i64 = 5000000000 tampilkan(n) }");
    assert_eq!(out, "5000000000\n");
}

#[test]
fn reg_negative_via_unary_neg() {
    let out = run_source("neg", "program R\nutama { nilai n = 10 tampilkan(-n) }");
    assert_eq!(out, "-10\n");
}

#[test]
fn reg_constant_folding_preserves_value() {
    let out = run_source("fold", "program R\nutama { tampilkan(2 + 3 * 4 - 1) }");
    assert_eq!(out, "13\n");
}

#[test]
fn reg_nested_block_comment_ignored() {
    let out = run_source(
        "comment",
        "program R\nutama { /* luar /* dalam */ luar */ tampilkan(\"ok\") }",
    );
    assert_eq!(out, "ok\n");
}
