use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_gs_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_gsc_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn basic_generic_struct_two_params() {
    let src = "program P\n\
        tipe Pasangan<T, U> { pertama: T kedua: U }\n\
        utama {\n\
          nilai p: Pasangan<angka, angka> = Pasangan { pertama: 3, kedua: 4 }\n\
          tampilkan(p.pertama + p.kedua)\n\
        }";
    let (out, code) = run_source("two", src);
    assert_eq!(out, "7\n");
    assert_eq!(code, 0);
}

#[test]
fn inference_from_return_position() {
    let src = "program P\n\
        tipe Pasangan<T, U> { pertama: T kedua: U }\n\
        fungsi buat(a: angka, b: teks) -> Pasangan<angka, teks> { kembalikan Pasangan { pertama: a, kedua: b } }\n\
        utama { nilai q = buat(10, \"x\") tampilkan(q.pertama) tampilkan(q.kedua) }";
    let (out, code) = run_source("infer", src);
    assert_eq!(out, "10\nx\n");
    assert_eq!(code, 0);
}

#[test]
fn multiple_instantiations() {
    let src = "program P\n\
        tipe Simpul<T> { data: T }\n\
        utama {\n\
          nilai a: Simpul<angka> = Simpul { data: 9 }\n\
          nilai b: Simpul<teks> = Simpul { data: \"hi\" }\n\
          tampilkan(a.data) tampilkan(b.data)\n\
        }";
    let (out, code) = run_source("multi", src);
    assert_eq!(out, "9\nhi\n");
    assert_eq!(code, 0);
}

#[test]
fn nested_generic_struct() {
    let src = "program P\n\
        tipe Pasangan<T, U> { pertama: T kedua: U }\n\
        tipe Titik<T> { x: T y: T }\n\
        utama {\n\
          nilai np: Pasangan<Titik<angka>, angka> = Pasangan { pertama: Titik { x: 7, y: 8 }, kedua: 100 }\n\
          tampilkan(np.pertama.x + np.pertama.y + np.kedua)\n\
        }";
    let (out, code) = run_source("nested", src);
    assert_eq!(out, "115\n");
    assert_eq!(code, 0);
}

#[test]
fn generic_struct_in_daftar() {
    let src = "program P\n\
        tipe Titik<T> { x: T y: T }\n\
        utama {\n\
          ubah ts: daftar<Titik<angka>> = daftar()\n\
          dorong(ambil_ubah(ts), Titik { x: 1, y: 2 })\n\
          dorong(ambil_ubah(ts), Titik { x: 3, y: 4 })\n\
          nilai t = elemen(ambil(ts), 1)\n\
          tampilkan(panjang(ambil(ts))) tampilkan(t.x + t.y)\n\
        }";
    let (out, code) = run_source("indaftar", src);
    assert_eq!(out, "2\n7\n");
    assert_eq!(code, 0);
}

#[test]
fn owned_field_makes_struct_owned() {
    let src = "program P\n\
        tipe Bungkus<T> { isi: T }\n\
        fungsi ukur(b: Bungkus<daftar<angka>>) -> angka { kembalikan panjang(ambil(b.isi)) }\n\
        utama {\n\
          ubah d: daftar<angka> = daftar()\n\
          dorong(ambil_ubah(d), 1) dorong(ambil_ubah(d), 2)\n\
          nilai b: Bungkus<daftar<angka>> = Bungkus { isi: d }\n\
          tampilkan(ukur(b))\n\
        }";
    let (out, code) = run_source("owned", src);
    assert_eq!(out, "2\n");
    assert_eq!(code, 0);
}

#[test]
fn use_after_move_owned_generic_struct() {
    let src = "program P\n\
        tipe Bungkus<T> { isi: T }\n\
        fungsi ukur(b: Bungkus<daftar<angka>>) -> angka { kembalikan panjang(ambil(b.isi)) }\n\
        utama {\n\
          ubah d: daftar<angka> = daftar()\n\
          nilai b: Bungkus<daftar<angka>> = Bungkus { isi: d }\n\
          nilai x = ukur(b)\n\
          nilai y = ukur(b)\n\
        }";
    let (err, code) = check_source("uam", src);
    assert_eq!(code, 1);
    assert!(err.contains("sudah dipindahkan"), "{err}");
}

#[test]
fn wrong_type_arg_count() {
    let src = "program P\n\
        tipe Pasangan<T, U> { a: T b: U }\n\
        utama { nilai p: Pasangan<angka> = Pasangan { a: 1, b: 2 } }";
    let (err, code) = check_source("arity", src);
    assert_eq!(code, 1);
    assert!(err.contains("butuh 2 argumen tipe"), "{err}");
}

#[test]
fn unknown_generic_type() {
    let src = "program P\n\
        utama { nilai p: TakAda<angka> = 1 }";
    let (err, code) = check_source("unknown", src);
    assert_eq!(code, 1);
    assert!(err.contains("tidak dikenal"), "{err}");
}

#[test]
fn field_type_mismatch_in_generic() {
    let src = "program P\n\
        tipe Simpul<T> { data: T }\n\
        utama { nilai s: Simpul<angka> = Simpul { data: \"bukan angka\" } }";
    let (err, code) = check_source("mismatch", src);
    assert_eq!(code, 1);
    assert!(err.contains("E0250") || err.contains("bertipe"), "{err}");
}
