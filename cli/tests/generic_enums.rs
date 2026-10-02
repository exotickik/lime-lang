use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_genenum_{}_{}", tag, std::process::id()));
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

const PILIHAN: &str = "enum Pilihan<T> { Ada(T), Kosong }\n";

#[test]
fn inferred_from_payload_argument() {
    let src = format!(
        "program G\n{PILIHAN}\
         fungsi buka(o: Pilihan<angka>, d: angka) -> angka {{\n\
           cocok o {{ Ada(n) => {{ kembalikan n }} Kosong => {{ kembalikan d }} }}\n\
           kembalikan -1\n\
         }}\n\
         utama {{ tampilkan(buka(Pilihan.Ada(42), 0)) }}"
    );
    let (out, code) = run_source("infer", &src);
    assert_eq!(out, "42\n");
    assert_eq!(code, 0);
}

#[test]
fn payloadless_variant_from_annotation() {
    let src = format!(
        "program G\n{PILIHAN}\
         fungsi buka(o: Pilihan<angka>, d: angka) -> angka {{\n\
           cocok o {{ Ada(n) => {{ kembalikan n }} Kosong => {{ kembalikan d }} }}\n\
           kembalikan -1\n\
         }}\n\
         utama {{ nilai k: Pilihan<angka> = Pilihan.Kosong tampilkan(buka(k, 99)) }}"
    );
    let (out, code) = run_source("annot", &src);
    assert_eq!(out, "99\n");
    assert_eq!(code, 0);
}

#[test]
fn returned_from_function() {
    let src = format!(
        "program G\n{PILIHAN}\
         fungsi cari(xs: &daftar<angka>, t: angka) -> Pilihan<angka> {{\n\
           untuk i dari 0 sampai panjang(xs) {{\n\
             jika elemen(xs, i) == t {{ kembalikan Pilihan.Ada(i) }}\n\
           }}\n\
           nilai k: Pilihan<angka> = Pilihan.Kosong\n\
           kembalikan k\n\
         }}\n\
         fungsi buka(o: Pilihan<angka>, d: angka) -> angka {{\n\
           cocok o {{ Ada(n) => {{ kembalikan n }} Kosong => {{ kembalikan d }} }}\n\
           kembalikan -1\n\
         }}\n\
         utama {{\n\
           ubah xs: daftar<angka> = daftar()\n\
           dorong(ambil_ubah(xs), 10) dorong(ambil_ubah(xs), 20) dorong(ambil_ubah(xs), 30)\n\
           tampilkan(buka(cari(ambil(xs), 20), -1))\n\
           tampilkan(buka(cari(ambil(xs), 77), -1))\n\
         }}"
    );
    let (out, code) = run_source("return", &src);
    assert_eq!(out, "1\n-1\n");
    assert_eq!(code, 0);
}

#[test]
fn two_type_parameters() {
    let src = "program G\n\
        enum Entah<A, B> { Kiri(A), Kanan(B) }\n\
        fungsi g(e: Entah<angka, boolean>) -> angka {\n\
          cocok e { Kiri(n) => { kembalikan n } Kanan(b) => { jika b { kembalikan 1 } kembalikan 0 } }\n\
          kembalikan -1\n\
        }\n\
        utama { tampilkan(g(Entah.Kiri(7))) tampilkan(g(Entah.Kanan(salah))) }";
    let (out, code) = run_source("twoparam", src);
    assert_eq!(out, "7\n0\n");
    assert_eq!(code, 0);
}

#[test]
fn distinct_instantiations() {
    let src = format!(
        "program G\n{PILIHAN}\
         utama {{\n\
           nilai p = Pilihan.Ada(3)\n\
           nilai q = Pilihan.Ada(benar)\n\
           cocok p {{ Ada(n) => {{ tampilkan(n) }} Kosong => {{}} }}\n\
           cocok q {{ Ada(b) => {{ tampilkan(b) }} Kosong => {{}} }}\n\
         }}"
    );
    let (out, code) = run_source("distinct", &src);
    assert_eq!(out, "3\nbenar\n");
    assert_eq!(code, 0);
}

#[test]
fn wrong_type_arg_arity_rejected() {
    let src = format!("program E\n{PILIHAN}utama {{ nilai x: Pilihan<angka, angka> = Pilihan.Kosong }}");
    let (code, err) = check_stderr("arity", &src);
    assert_ne!(code, 0);
    assert!(err.contains("argumen tipe"), "stderr: {err}");
}

#[test]
fn cannot_infer_payloadless_without_annotation() {
    let src = format!("program E\n{PILIHAN}utama {{ nilai x = Pilihan.Kosong tampilkan(1) }}");
    let (code, err) = check_stderr("noinfer", &src);
    assert_ne!(code, 0);
    assert!(err.contains("menyimpulkan"), "stderr: {err}");
}

#[test]
fn bare_payload_variant_rejected() {
    let src = format!("program E\n{PILIHAN}utama {{ nilai x = Pilihan.Ada }}");
    let (code, err) = check_stderr("bare", &src);
    assert_ne!(code, 0);
    assert!(err.contains("E0220"), "stderr: {err}");
}

#[test]
fn generic_enum_owned_type_argument_works() {
    let src = format!(
        "program G\n{PILIHAN}\
         utama {{\n\
           ubah xs: daftar<angka> = daftar()\n\
           dorong(ambil_ubah(xs), 10) dorong(ambil_ubah(xs), 20)\n\
           nilai p: Pilihan<daftar<angka>> = Pilihan.Ada(xs)\n\
           cocok p {{ Ada(v) => {{ tampilkan(panjang(ambil(v))) }} Kosong => {{ tampilkan(0) }} }}\n\
         }}"
    );
    let (out, code) = run_source("owned_targ", &src);
    assert_eq!(out, "2\n");
    assert_eq!(code, 0);
}

#[test]
fn generic_enum_owned_unused_payload_dropped() {
    let src = format!(
        "program G\n{PILIHAN}\
         utama {{\n\
           ubah xs: daftar<angka> = daftar()\n\
           dorong(ambil_ubah(xs), 1)\n\
           nilai p: Pilihan<daftar<angka>> = Pilihan.Ada(xs)\n\
           cocok p {{ Ada => {{ tampilkan(-5) }} Kosong => {{ tampilkan(0) }} }}\n\
         }}"
    );
    let (out, code) = run_source("owned_unused", &src);
    assert_eq!(out, "-5\n");
    assert_eq!(code, 0);
}
