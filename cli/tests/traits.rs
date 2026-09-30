use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_trait_{}_{}", tag, std::process::id()));
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

const SHAPES: &str = "program Bentuk\n\
    tipe Persegi { sisi: angka }\n\
    tipe Lingkaran { jari: angka }\n\
    sifat Luas { fungsi luas(diri: &Diri) -> angka }\n\
    implementasi Luas untuk Persegi { fungsi luas(diri: &Diri) -> angka { kembalikan diri.sisi * diri.sisi } }\n\
    implementasi Luas untuk Lingkaran { fungsi luas(diri: &Diri) -> angka { kembalikan 3 * diri.jari * diri.jari } }\n";

#[test]
fn trait_method_dispatch() {
    let src = format!(
        "{SHAPES}utama {{ nilai p = Persegi {{ sisi: 4 }} nilai c = Lingkaran {{ jari: 5 }} \
         tampilkan(p.luas()) tampilkan(c.luas()) }}"
    );
    let (out, code) = run_source("dispatch", &src);
    assert_eq!(out, "16\n75\n");
    assert_eq!(code, 0);
}

#[test]
fn inherent_mutable_method_mutates_in_place() {
    let src = "program M\n\
        tipe Hitung { n: angka }\n\
        implementasi Hitung {\n\
            fungsi naik(diri: &ubah Diri) { diri.n = diri.n + 1 }\n\
            fungsi nilai_sekarang(diri: &Diri) -> angka { kembalikan diri.n }\n\
        }\n\
        utama { ubah h = Hitung { n: 10 } h.naik() h.naik() tampilkan(h.nilai_sekarang()) }";
    let (out, _) = run_source("mutmethod", src);
    assert_eq!(out, "12\n");
}

#[test]
fn trait_bounded_generic_static_dispatch() {
    let src = format!(
        "{SHAPES}fungsi cetak<T: Luas>(v: &T) {{ tampilkan(v.luas()) }}\n\
         utama {{ nilai p = Persegi {{ sisi: 6 }} nilai c = Lingkaran {{ jari: 2 }} \
         cetak(ambil(p)) cetak(ambil(c)) }}"
    );
    let (out, _) = run_source("boundgen", &src);
    assert_eq!(out, "36\n12\n");

    let ir = emit_ir("boundgen", &src);
    assert_eq!(ir.matches("call ptr @malloc").count(), 0, "traits add no allocation");
    assert!(ir.contains("$Luas$luas"), "impl methods lower to ordinary functions");
    assert_eq!(
        ir.matches("define void @cetak$").count(),
        2,
        "the bounded generic is monomorphized once per concrete type (static dispatch)"
    );
    assert!(!ir.contains("vtable") && !ir.contains("@llvm.load.relative"), "no dynamic dispatch");
}

#[test]
fn method_returning_value_used_in_expression() {
    let src = format!(
        "{SHAPES}utama {{ nilai p = Persegi {{ sisi: 3 }} nilai c = Lingkaran {{ jari: 1 }} \
         tampilkan(p.luas() + c.luas()) }}"
    );
    let (out, _) = run_source("expr", &src);
    assert_eq!(out, "12\n");
}

#[test]
fn method_with_extra_arguments() {
    let src = "program A\n\
        tipe Akun { saldo: angka }\n\
        implementasi Akun {\n\
            fungsi setor(diri: &ubah Diri, jumlah: angka) { diri.saldo = diri.saldo + jumlah }\n\
        }\n\
        utama { ubah a = Akun { saldo: 100 } a.setor(50) tampilkan(a.saldo) }";
    let (out, _) = run_source("args", src);
    assert_eq!(out, "150\n");
}

#[test]
fn bound_not_satisfied_is_error() {
    let src = format!(
        "{SHAPES}tipe Kosong {{ a: angka }}\n\
         fungsi cetak<T: Luas>(v: &T) {{ tampilkan(v.luas()) }}\n\
         utama {{ nilai k = Kosong {{ a: 1 }} cetak(ambil(k)) }}"
    );
    let (code, err) = check_stderr("nobound", &src);
    assert_eq!(code, 1);
    assert!(err.contains("E02A4"), "got: {err}");
}

#[test]
fn method_not_found_is_error() {
    let src = "program A\ntipe A { x: angka }\nutama { nilai a = A { x: 1 } tampilkan(a.tiada()) }";
    let (code, err) = check_stderr("nomethod", src);
    assert_eq!(code, 1);
    assert!(err.contains("E02A2"), "got: {err}");
}

#[test]
fn incomplete_impl_is_error() {
    let src = "program A\ntipe A { x: angka }\n\
        sifat S { fungsi m(diri: &Diri) -> angka }\n\
        implementasi S untuk A { }\nutama { }";
    let (code, err) = check_stderr("incomplete", src);
    assert_eq!(code, 1);
    assert!(err.contains("E02A1"), "got: {err}");
}
