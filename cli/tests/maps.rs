use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_map_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_mapc_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn put_get_overwrite_len() {
    let src = "program P\n\
        utama {\n\
          ubah m: peta<teks, angka> = peta()\n\
          taruh(ambil_ubah(m), \"satu\", 1)\n\
          taruh(ambil_ubah(m), \"dua\", 2)\n\
          tampilkan(ambil_peta(ambil(m), \"satu\", 0))\n\
          tampilkan(ambil_peta(ambil(m), \"dua\", 0))\n\
          taruh(ambil_ubah(m), \"satu\", 11)\n\
          tampilkan(ambil_peta(ambil(m), \"satu\", 0))\n\
          tampilkan(panjang(ambil(m)))\n\
        }";
    let (out, code) = run_source("basic", src);
    assert_eq!(out, "1\n2\n11\n2\n");
    assert_eq!(code, 0);
}

#[test]
fn absent_returns_default() {
    let src = "program P\n\
        utama {\n\
          ubah m: peta<teks, angka> = peta()\n\
          taruh(ambil_ubah(m), \"ada\", 7)\n\
          tampilkan(ambil_peta(ambil(m), \"hilang\", 0 - 9))\n\
        }";
    let (out, _) = run_source("absent", src);
    assert_eq!(out, "-9\n");
}

#[test]
fn ada_contains() {
    let src = "program P\n\
        utama {\n\
          ubah m: peta<teks, angka> = peta()\n\
          taruh(ambil_ubah(m), \"k\", 1)\n\
          jika ada(ambil(m), \"k\") { tampilkan(1) } lain { tampilkan(0) }\n\
          jika ada(ambil(m), \"z\") { tampilkan(1) } lain { tampilkan(0) }\n\
        }";
    let (out, _) = run_source("ada", src);
    assert_eq!(out, "1\n0\n");
}

#[test]
fn empty_map_is_safe() {
    let src = "program P\n\
        utama {\n\
          ubah m: peta<teks, angka> = peta()\n\
          tampilkan(ambil_peta(ambil(m), \"x\", 42))\n\
          jika ada(ambil(m), \"x\") { tampilkan(1) } lain { tampilkan(0) }\n\
          tampilkan(panjang(ambil(m)))\n\
        }";
    let (out, code) = run_source("empty", src);
    assert_eq!(out, "42\n0\n0\n");
    assert_eq!(code, 0);
}

#[test]
fn growth_preserves_all_entries() {
    let src = "program P\n\
        utama {\n\
          ubah m: peta<teks, angka> = peta()\n\
          taruh(ambil_ubah(m), \"a\", 1)\n\
          taruh(ambil_ubah(m), \"b\", 2)\n\
          taruh(ambil_ubah(m), \"c\", 3)\n\
          taruh(ambil_ubah(m), \"d\", 4)\n\
          taruh(ambil_ubah(m), \"e\", 5)\n\
          taruh(ambil_ubah(m), \"f\", 6)\n\
          taruh(ambil_ubah(m), \"g\", 7)\n\
          taruh(ambil_ubah(m), \"h\", 8)\n\
          taruh(ambil_ubah(m), \"i\", 9)\n\
          taruh(ambil_ubah(m), \"j\", 10)\n\
          tampilkan(panjang(ambil(m)))\n\
          tampilkan(ambil_peta(ambil(m), \"a\", 0))\n\
          tampilkan(ambil_peta(ambil(m), \"f\", 0))\n\
          tampilkan(ambil_peta(ambil(m), \"j\", 0))\n\
        }";
    let (out, code) = run_source("growth", src);
    assert_eq!(out, "10\n1\n6\n10\n");
    assert_eq!(code, 0);
}

#[test]
fn passed_by_reference() {
    let src = "program P\n\
        fungsi cari(m: &peta<teks, angka>, k: teks) -> angka { kembalikan ambil_peta(m, k, 0 - 1) }\n\
        utama {\n\
          ubah m: peta<teks, angka> = peta()\n\
          taruh(ambil_ubah(m), \"umur\", 25)\n\
          tampilkan(cari(ambil(m), \"umur\"))\n\
          tampilkan(cari(ambil(m), \"nil\"))\n\
        }";
    let (out, _) = run_source("byref", src);
    assert_eq!(out, "25\n-1\n");
}

#[test]
fn non_teks_key_is_error() {
    let src = "program P\n\
        utama { ubah m: peta<angka, angka> = peta() taruh(ambil_ubah(m), 1, 2) }";
    let (err, code) = check_source("badkey", src);
    assert_eq!(code, 1);
    assert!(err.contains("kunci 'teks'"), "{err}");
}

#[test]
fn owned_value_is_error() {
    let src = "program P\n\
        utama { ubah m: peta<teks, daftar<angka>> = peta() tampilkan(1) }";
    let (err, code) = check_source("ownval", src);
    assert_eq!(code, 1);
    assert!(err.contains("bertipe milik"), "{err}");
}

#[test]
fn peta_moves_like_owned() {
    let src = "program P\n\
        fungsi pakai(m: peta<teks, angka>) -> angka { kembalikan panjang(ambil(m)) }\n\
        utama { ubah m: peta<teks, angka> = peta() nilai n = pakai(m) tampilkan(panjang(ambil(m))) }";
    let (err, code) = check_source("move", src);
    assert_eq!(code, 1);
    assert!(err.contains("sudah dipindahkan"), "{err}");
}
