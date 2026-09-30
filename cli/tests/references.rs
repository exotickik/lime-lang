use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_ref_{}_{}", tag, std::process::id()));
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

fn emit_ir(tag: &str, src: &str) -> String {
    let path = dir(tag).join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("emit-ir").arg(&path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn mutation_through_mutable_reference() {
    let src = "program R\n\
        fungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
        utama { ubah n = 10 naik(ambil_ubah(n)) naik(ambil_ubah(n)) tampilkan(n) }";
    let (out, code) = run_source("mut", src);
    assert_eq!(out, "12\n");
    assert_eq!(code, 0);
}

#[test]
fn read_through_shared_reference() {
    let src = "program R\n\
        fungsi baca(x: &angka) -> angka { kembalikan *x }\n\
        utama { nilai n = 7 tampilkan(baca(ambil(n))) }";
    let (out, _) = run_source("read", src);
    assert_eq!(out, "7\n");
}

#[test]
fn multiple_shared_references_at_once() {
    let src = "program R\n\
        fungsi jumlah(a: &angka, b: &angka) -> angka { kembalikan *a + *b }\n\
        utama { nilai n = 21 tampilkan(jumlah(ambil(n), ambil(n))) }";
    let (out, _) = run_source("multi", src);
    assert_eq!(out, "42\n");
}

#[test]
fn reference_to_struct_field_mutates_in_place() {
    let src = "program R\n\
        tipe Titik { x: angka, y: angka }\n\
        fungsi naik(v: &ubah angka) { *v = *v + 5 }\n\
        utama { ubah p = Titik { x: 1, y: 2 } naik(ambil_ubah(p.x)) tampilkan(p.x) tampilkan(p.y) }";
    let (out, _) = run_source("field", src);
    assert_eq!(out, "6\n2\n");
}

#[test]
fn reference_to_array_element_mutates_in_place() {
    let src = "program R\n\
        fungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
        utama { ubah a = [10, 20, 30] naik(ambil_ubah(a[1])) tampilkan(a[0]) tampilkan(a[1]) }";
    let (out, _) = run_source("arr", src);
    assert_eq!(out, "10\n21\n");
}

#[test]
fn borrow_of_box_does_not_move_or_double_free() {
    let src = "program R\n\
        fungsi lihat(b: &kotak<angka>) -> angka { kembalikan isi(b) }\n\
        utama { nilai b = kotak(9) tampilkan(lihat(ambil(b))) tampilkan(isi(b)) }";
    let (out, code) = run_source("boxref", src);
    assert_eq!(out, "9\n9\n");
    assert_eq!(code, 0);
    let ir = emit_ir("boxref", src);
    assert_eq!(ir.matches("call ptr @malloc").count(), 1, "borrow must not allocate");
    assert_eq!(ir.matches("call void @free").count(), 1, "box freed exactly once");
}

#[test]
fn generic_function_over_references() {
    let src = "program R\n\
        fungsi set<T>(x: &ubah T, v: T) { *x = v }\n\
        fungsi baca<T>(x: &T) -> T { kembalikan *x }\n\
        utama { ubah n = 1 set(ambil_ubah(n), 8) tampilkan(baca(ambil(n))) }";
    let (out, _) = run_source("generic", src);
    assert_eq!(out, "8\n");
}

#[test]
fn references_are_zero_cost_no_allocation() {
    let src = "program R\n\
        fungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
        utama { ubah n = 0 naik(ambil_ubah(n)) tampilkan(n) }";
    let ir = emit_ir("zerocost", src);
    assert_eq!(ir.matches("call ptr @malloc").count(), 0, "references must not allocate");
    assert!(ir.contains("store i64"), "mutation lowers to a native store");
    assert!(ir.contains("load i64, ptr"), "deref lowers to a native load");
}

#[test]
fn mutate_while_borrowed_is_error() {
    let src = "program R\nutama { ubah n = 5 nilai r = ambil(n) n = 10 tampilkan(*r) }";
    let (code, err) = check_stderr("e_mut", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0296"), "got: {err}");
}

#[test]
fn mutable_borrow_conflict_is_error() {
    let src = "program R\nutama { ubah n = 5 nilai a = ambil_ubah(n) nilai b = ambil(n) tampilkan(*a) }";
    let (code, err) = check_stderr("e_conf", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0295"), "got: {err}");
}

#[test]
fn mutable_borrow_of_immutable_is_error() {
    let src = "program R\n\
        fungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
        utama { nilai n = 5 naik(ambil_ubah(n)) }";
    let (code, err) = check_stderr("e_imm", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0291"), "got: {err}");
}

#[test]
fn returning_reference_is_error() {
    let src = "program R\nfungsi bocor(n: angka) -> &angka { kembalikan ambil(n) }\nutama { }";
    let (code, err) = check_stderr("e_esc", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0297"), "got: {err}");
}

#[test]
fn reference_crossing_ffi_is_error() {
    let src = "program R\nekstern fungsi c_pakai(x: &angka) -> angka\nutama { }";
    let (code, err) = check_stderr("e_ffi", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0298"), "got: {err}");
}

#[test]
fn write_through_shared_reference_is_error() {
    let src = "program R\nfungsi f(x: &angka) { *x = 1 }\nutama { }";
    let (code, err) = check_stderr("e_shared", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0294"), "got: {err}");
}
