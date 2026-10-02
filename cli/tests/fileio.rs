use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_io_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn write_then_read_roundtrip() {
    let tmp = dir("rt").join("data.txt");
    let tmp_s = tmp.to_string_lossy().replace('\\', "/");
    let src = format!(
        "program P\ngunakan Teks\nutama {{\n\
            ubah out: daftar<bita> = daftar()\n\
            tambah_teks(ambil_ubah(out), \"baris\")\n\
            baris_baru(ambil_ubah(out))\n\
            tambah_angka(ambil_ubah(out), 999)\n\
            tampilkan(tulis_file(\"{tmp_s}\", ambil(out)))\n\
            ubah inp: daftar<bita> = daftar()\n\
            tampilkan(baca_file(\"{tmp_s}\", ambil_ubah(inp)))\n\
            tampilkan(panjang(ambil(inp)))\n\
            cetak_bita(ambil(inp))\n\
        }}"
    );
    let (out, code) = run_source("rt", &src);
    assert_eq!(out, "benar\nbenar\n9\nbaris\n999");
    assert_eq!(code, 0);
    let on_disk = std::fs::read_to_string(&tmp).unwrap();
    assert_eq!(on_disk, "baris\n999");
}

#[test]
fn reading_missing_file_returns_false() {
    let missing = dir("miss").join("does_not_exist_zzz.txt");
    let missing_s = missing.to_string_lossy().replace('\\', "/");
    let src = format!(
        "program P\nutama {{\n\
            ubah inp: daftar<bita> = daftar()\n\
            tampilkan(baca_file(\"{missing_s}\", ambil_ubah(inp)))\n\
        }}"
    );
    let (out, _) = run_source("miss", &src);
    assert_eq!(out, "salah\n");
}

#[test]
fn count_bytes_of_read_file() {
    let tmp = dir("cnt").join("bytes.bin");
    std::fs::write(&tmp, "halo dunia").unwrap();
    let tmp_s = tmp.to_string_lossy().replace('\\', "/");
    let src = format!(
        "program P\nutama {{\n\
            ubah inp: daftar<bita> = daftar()\n\
            nilai ok = baca_file(\"{tmp_s}\", ambil_ubah(inp))\n\
            tampilkan(panjang(ambil(inp)))\n\
            tampilkan(elemen(ambil(inp), 0) sebagai angka)\n\
        }}"
    );
    let (out, _) = run_source("cnt", &src);
    assert_eq!(out, "10\n104\n");
}
