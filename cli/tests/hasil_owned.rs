use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_ohr_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_ohc_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}

const BUAT: &str = "fungsi buat(n: angka) -> Hasil<daftar<angka>, teks> {\n\
      jika n < 0 { kembalikan gagal(\"negatif\") }\n\
      ubah d: daftar<angka> = daftar()\n\
      ubah i: angka = 0\n\
      selama i < n { dorong(ambil_ubah(d), i) i = i + 1 }\n\
      kembalikan sukses(d)\n\
    }\n";

#[test]
fn coba_success_extracts_owned_payload() {
    let src = format!(
        "program P\n{BUAT}\
        fungsi jumlah(n: angka) -> Hasil<angka, teks> {{\n\
          nilai d = coba buat(n)\n\
          ubah t: angka = 0\n\
          ubah i: angka = 0\n\
          selama i < panjang(ambil(d)) {{ t = t + elemen(ambil(d), i) i = i + 1 }}\n\
          kembalikan sukses(t)\n\
        }}\n\
        utama {{ tampilkan(nilai_atau(jumlah(5), 0 - 1)) }}"
    );
    let (out, code) = run_source("coba_ok", &src);
    assert_eq!(out, "10\n");
    assert_eq!(code, 0);
}

#[test]
fn coba_propagates_error() {
    let src = format!(
        "program P\n{BUAT}\
        fungsi jumlah(n: angka) -> Hasil<angka, teks> {{\n\
          nilai d = coba buat(n)\n\
          kembalikan sukses(panjang(ambil(d)))\n\
        }}\n\
        utama {{ tampilkan(nilai_atau(jumlah(0 - 1), 0 - 99)) tampilkan(gagal_atau(jumlah(0 - 1), \"?\")) }}"
    );
    let (out, code) = run_source("coba_err", &src);
    assert_eq!(out, "-99\nnegatif\n");
    assert_eq!(code, 0);
}

#[test]
fn dropping_unused_owned_results_is_clean() {
    let src = format!(
        "program P\n{BUAT}\
        utama {{\n\
          nilai a = buat(4)\n\
          nilai b = buat(0 - 1)\n\
          tampilkan(1)\n\
        }}"
    );
    let (out, code) = run_source("drop", &src);
    assert_eq!(out, "1\n");
    assert_eq!(code, 0);
}

#[test]
fn owned_error_payload_drops() {
    let src = "program P\n\
        fungsi f(ok: boolean) -> Hasil<angka, daftar<bita>> {\n\
          jika ok { kembalikan sukses(1) }\n\
          ubah p: daftar<bita> = daftar()\n\
          dorong(ambil_ubah(p), 65 sebagai bita)\n\
          kembalikan gagal(p)\n\
        }\n\
        utama { nilai a = f(benar) nilai b = f(salah) tampilkan(9) }";
    let (out, code) = run_source("owned_err", src);
    assert_eq!(out, "9\n");
    assert_eq!(code, 0);
}

#[test]
fn nested_owned_payload_peta() {
    let src = "program P\n\
        fungsi muat(ok: boolean) -> Hasil<peta<teks, angka>, teks> {\n\
          jika bukan ok { kembalikan gagal(\"x\") }\n\
          ubah m: peta<teks, angka> = peta()\n\
          taruh(ambil_ubah(m), \"k\", 77)\n\
          kembalikan sukses(m)\n\
        }\n\
        fungsi baca(ok: boolean) -> Hasil<angka, teks> {\n\
          nilai m = coba muat(ok)\n\
          kembalikan sukses(ambil_peta(ambil(m), \"k\", 0))\n\
        }\n\
        utama { tampilkan(nilai_atau(baca(benar), 0)) tampilkan(nilai_atau(baca(salah), 0 - 1)) nilai d = muat(benar) tampilkan(5) }";
    let (out, code) = run_source("nested", src);
    assert_eq!(out, "77\n-1\n5\n");
    assert_eq!(code, 0);
}

#[test]
fn use_after_move_of_owned_hasil_rejected() {
    let src = format!(
        "program P\n{BUAT}\
        fungsi f(n: angka) -> Hasil<angka, teks> {{\n\
          nilai r = buat(n)\n\
          nilai a = coba r\n\
          nilai b = coba r\n\
          kembalikan sukses(panjang(ambil(a)) + panjang(ambil(b)))\n\
        }}\n\
        utama {{ nilai x = f(2) }}"
    );
    let (err, code) = check_source("uam", &src);
    assert_eq!(code, 1);
    assert!(err.contains("sudah dipindahkan"), "{err}");
}

#[test]
fn copy_payload_inspection_still_works() {
    let src = "program P\n\
        fungsi g(n: angka) -> Hasil<angka, teks> {\n\
          jika n < 0 { kembalikan gagal(\"neg\") }\n\
          kembalikan sukses(n * 2)\n\
        }\n\
        utama {\n\
          jika berhasil(g(3)) { tampilkan(1) }\n\
          tampilkan(nilai_atau(g(3), 0))\n\
          tampilkan(nilai_atau(g(0 - 1), 0 - 5))\n\
        }";
    let (out, code) = run_source("copyinspect", src);
    assert_eq!(out, "1\n6\n-5\n");
    assert_eq!(code, 0);
}
