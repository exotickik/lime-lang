use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_struct_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn construct_and_read_fields() {
    let src = "program S\n\
        tipe Pemain { nama: teks\n skor: i32 }\n\
        utama { nilai p = Pemain { nama: \"Budi\", skor: 100i32 } tampilkan(p.nama) tampilkan(p.skor) }";
    let (out, code) = run_source("read", src);
    assert_eq!(out, "Budi\n100\n");
    assert_eq!(code, 0);
}

#[test]
fn field_mutation() {
    let src = "program S\n\
        tipe Kotak { lebar: i32 }\n\
        utama { ubah k = Kotak { lebar: 5i32 } k.lebar = 42i32 tampilkan(k.lebar) }";
    let (out, _) = run_source("mut", src);
    assert_eq!(out, "42\n");
}

#[test]
fn nested_struct_access_and_mutation() {
    let src = "program S\n\
        tipe Titik { x: i32\n y: i32 }\n\
        tipe Garis { awal: Titik\n akhir: Titik }\n\
        utama {\n\
          ubah g = Garis { awal: Titik { x: 0i32, y: 0i32 }, akhir: Titik { x: 3i32, y: 4i32 } }\n\
          tampilkan(g.akhir.x)\n\
          g.awal.x = 10i32\n\
          tampilkan(g.awal.x)\n\
        }";
    let (out, _) = run_source("nested", src);
    assert_eq!(out, "3\n10\n");
}

#[test]
fn struct_as_parameter_and_return() {
    let src = "program S\n\
        tipe Titik { x: i32\n y: i32 }\n\
        fungsi geser(t: Titik, d: i32) -> Titik { kembalikan Titik { x: t.x + d, y: t.y + d } }\n\
        fungsi jumlah(t: Titik) -> i32 { kembalikan t.x + t.y }\n\
        utama { nilai a = Titik { x: 1i32, y: 2i32 } nilai b = geser(a, 10i32) tampilkan(jumlah(b)) }";
    let (out, _) = run_source("param", src);
    assert_eq!(out, "23\n");
}

#[test]
fn struct_default_i64_fields() {
    let src = "program S\n\
        tipe Akun { saldo: angka }\n\
        utama { ubah a = Akun { saldo: 1000 } a.saldo = a.saldo + 500 tampilkan(a.saldo) }";
    let (out, _) = run_source("i64f", src);
    assert_eq!(out, "1500\n");
}

#[test]
fn struct_literal_not_confused_with_block_in_if() {
    let src = "program S\n\
        utama { nilai x = 5 jika x > 3 { tampilkan(\"lebih\") } lain { tampilkan(\"kurang\") } }";
    let (out, _) = run_source("ifblock", src);
    assert_eq!(out, "lebih\n");
}
