use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_str_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn numeric_casts() {
    let src = "program P\nutama {\n\
        tampilkan(300 sebagai bita)\n\
        tampilkan(65 sebagai karakter)\n\
        tampilkan((3 sebagai f64) / 2)\n\
        nilai b = 300 sebagai bita\n\
        tampilkan(b sebagai angka)\n\
    }";
    let (out, code) = run_source("cast", src);
    assert_eq!(out, "44\nA\n1.5\n44\n");
    assert_eq!(code, 0);
}

#[test]
fn build_string_with_stdlib() {
    let src = "program P\ngunakan Teks\nutama {\n\
        ubah buf: daftar<bita> = daftar()\n\
        tambah_teks(ambil_ubah(buf), \"jawaban = \")\n\
        tambah_angka(ambil_ubah(buf), 42)\n\
        baris_baru(ambil_ubah(buf))\n\
        tambah_angka(ambil_ubah(buf), 0 - 123)\n\
        baris_baru(ambil_ubah(buf))\n\
        cetak_bita(ambil(buf))\n\
    }";
    let (out, _) = run_source("build", src);
    assert_eq!(out, "jawaban = 42\n-123\n");
}

#[test]
fn read_bytes_of_a_string() {
    let src = "program P\nutama {\n\
        nilai s = \"ABC\"\n\
        tampilkan(bita_pada(s, 0) sebagai angka)\n\
        tampilkan(bita_pada(s, 2) sebagai angka)\n\
        tampilkan(panjang(s))\n\
    }";
    let (out, _) = run_source("read", src);
    assert_eq!(out, "65\n67\n3\n");
}

#[test]
fn reverse_a_string_via_byte_buffer() {
    let src = "program P\ngunakan Teks\nutama {\n\
        nilai s = \"halo\"\n\
        ubah buf: daftar<bita> = daftar()\n\
        ubah i = panjang(s) - 1\n\
        selama i >= 0 { dorong(ambil_ubah(buf), bita_pada(s, i)) i = i - 1 }\n\
        baris_baru(ambil_ubah(buf))\n\
        cetak_bita(ambil(buf))\n\
    }";
    let (out, _) = run_source("rev", src);
    assert_eq!(out, "olah\n");
}
