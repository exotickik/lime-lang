use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_std_{}_{}", tag, std::process::id()));
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

#[test]
fn teks_sama_and_awalan() {
    let src = "program T\n\
        gunakan Teks\n\
        utama {\n\
          ubah a: daftar<bita> = daftar()\n\
          tambah_teks(ambil_ubah(a), \"fungsi\")\n\
          jika sama_teks(ambil(a), \"fungsi\") { tampilkan(1) } lain { tampilkan(0) }\n\
          jika sama_teks(ambil(a), \"beda\") { tampilkan(1) } lain { tampilkan(0) }\n\
          jika awalan(ambil(a), \"fun\") { tampilkan(1) } lain { tampilkan(0) }\n\
          jika awalan(ambil(a), \"xyz\") { tampilkan(1) } lain { tampilkan(0) }\n\
        }";
    let (out, code) = run_source("teks_sama", src);
    assert_eq!(out, "1\n0\n1\n0\n");
    assert_eq!(code, 0);
}

#[test]
fn teks_cari_and_potong() {
    let src = "program T\n\
        gunakan Teks\n\
        utama {\n\
          ubah a: daftar<bita> = daftar()\n\
          tambah_teks(ambil_ubah(a), \"fungsi\")\n\
          tampilkan(cari_bita(ambil(a), 110 sebagai bita))\n\
          tampilkan(cari_bita(ambil(a), 122 sebagai bita))\n\
          ubah sub: daftar<bita> = potong(ambil(a), 0, 3)\n\
          jika sama_teks(ambil(sub), \"fun\") { tampilkan(1) } lain { tampilkan(0) }\n\
          tampilkan(panjang(ambil(sub)))\n\
        }";
    let (out, code) = run_source("teks_cari", src);
    assert_eq!(out, "2\n-1\n1\n3\n");
    assert_eq!(code, 0);
}

#[test]
fn teks_sama_bita() {
    let src = "program T\n\
        gunakan Teks\n\
        utama {\n\
          ubah a: daftar<bita> = daftar()\n\
          tambah_teks(ambil_ubah(a), \"abc\")\n\
          ubah b: daftar<bita> = daftar()\n\
          tambah_teks(ambil_ubah(b), \"abc\")\n\
          ubah c: daftar<bita> = daftar()\n\
          tambah_teks(ambil_ubah(c), \"abd\")\n\
          jika sama_bita(ambil(a), ambil(b)) { tampilkan(1) } lain { tampilkan(0) }\n\
          jika sama_bita(ambil(a), ambil(c)) { tampilkan(1) } lain { tampilkan(0) }\n\
        }";
    let (out, _) = run_source("teks_samabita", src);
    assert_eq!(out, "1\n0\n");
}
