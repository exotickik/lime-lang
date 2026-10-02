use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_own_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn alloc_balance(tag: &str, src: &str) -> (usize, usize) {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("emit-ir").arg(&path).output().unwrap();
    let ir = String::from_utf8_lossy(&out.stdout);
    let mallocs = ir.matches("call ptr @malloc").count();
    let frees = ir.matches("call void @free").count();
    (mallocs, frees)
}

#[test]
fn box_create_read_and_drop() {
    let src = "program O\nutama { nilai b = kotak(42) tampilkan(isi(b)) }";
    let (out, code) = run_source("basic", src);
    assert_eq!(out, "42\n");
    assert_eq!(code, 0);
    let (m, f) = alloc_balance("basic", src);
    assert_eq!((m, f), (1, 1), "one malloc, one free");
}

#[test]
fn ownership_transfer_through_call_and_return() {
    let src = "program O\n\
        fungsi konsumsi(k: kotak<i64>) -> i64 { kembalikan isi(k) }\n\
        fungsi buat(n: i64) -> kotak<i64> { kembalikan kotak(n) }\n\
        utama { nilai a = kotak(10) tampilkan(konsumsi(a)) nilai b = buat(99) tampilkan(isi(b)) }";
    let (out, _) = run_source("transfer", src);
    assert_eq!(out, "10\n99\n");
    let (m, f) = alloc_balance("transfer", src);
    assert_eq!(m, f, "allocations must balance");
    assert_eq!(m, 2);
}

#[test]
fn box_dropped_at_block_scope_exit() {
    let src = "program O\n\
        utama { ubah t = 0 jika benar { nilai b = kotak(100) t = t + isi(b) } tampilkan(t) }";
    let (out, _) = run_source("block", src);
    assert_eq!(out, "100\n");
    let (m, f) = alloc_balance("block", src);
    assert_eq!((m, f), (1, 1));
}

#[test]
fn owned_struct_recursive_drop() {
    let src = "program O\n\
        tipe Simpul { data: kotak<i64> }\n\
        utama { nilai s = Simpul { data: kotak(7) } tampilkan(isi(s.data)) }";
    let (out, _) = run_source("struct", src);
    assert_eq!(out, "7\n");
    let (m, f) = alloc_balance("struct", src);
    assert_eq!((m, f), (1, 1), "struct's box freed via recursive drop");
}

#[test]
fn box_created_each_loop_iteration_balanced() {
    let src = "program O\n\
        utama { ubah s = 0 untuk i dari 0 sampai 4 { nilai b = kotak(i) s = s + isi(b) } tampilkan(s) }";
    let (out, _) = run_source("loop", src);
    assert_eq!(out, "6\n");
    let (m, f) = alloc_balance("loop", src);
    assert_eq!(m, f, "per-iteration alloc and free must balance");
}

#[test]
fn generic_box_multiple_instances() {
    let src = "program O\n\
        fungsi bungkus<T>(x: T) -> kotak<T> { kembalikan kotak(x) }\n\
        utama { nilai a = bungkus(42) tampilkan(isi(a)) nilai b = bungkus(7i32) tampilkan(isi(b)) }";
    let (out, _) = run_source("generic", src);
    assert_eq!(out, "42\n7\n");
    let (m, f) = alloc_balance("generic", src);
    assert_eq!((m, f), (2, 2));
}

#[test]
fn use_after_move_is_compile_error() {
    let path = dir("uam").join("prog.exo");
    std::fs::write(
        &path,
        "program O\nfungsi k(b: kotak<i64>) -> i64 { kembalikan isi(b) }\nutama { nilai b = kotak(1) nilai c = b tampilkan(k(b)) }",
    )
    .unwrap();
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("E0280"));
}
