use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_enum_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn prints_variant_names() {
    let src = "program E\n\
        enum Hasil { Berhasil, Gagal, Menunggu }\n\
        utama { tampilkan(Hasil.Berhasil) tampilkan(Hasil.Gagal) tampilkan(Hasil.Menunggu) }";
    let (out, code) = run_source("names", src);
    assert_eq!(out, "Berhasil\nGagal\nMenunggu\n");
    assert_eq!(code, 0);
}

#[test]
fn enum_equality_branches() {
    let src = "program E\n\
        enum Arah { Utara, Selatan }\n\
        utama {\n\
          nilai a = Arah.Utara\n\
          jika a == Arah.Utara { tampilkan(\"utara\") } lain { tampilkan(\"lain\") }\n\
          jika a != Arah.Selatan { tampilkan(\"bukan selatan\") }\n\
        }";
    let (out, _) = run_source("eq", src);
    assert_eq!(out, "utara\nbukan selatan\n");
}

#[test]
fn enum_as_param_and_return() {
    let src = "program E\n\
        enum Sinyal { Merah, Kuning, Hijau }\n\
        fungsi berikut(s: Sinyal) -> Sinyal {\n\
          jika s == Sinyal.Merah { kembalikan Sinyal.Hijau }\n\
          jika s == Sinyal.Hijau { kembalikan Sinyal.Kuning }\n\
          kembalikan Sinyal.Merah\n\
        }\n\
        utama { tampilkan(berikut(Sinyal.Merah)) tampilkan(berikut(Sinyal.Hijau)) }";
    let (out, _) = run_source("param", src);
    assert_eq!(out, "Hijau\nKuning\n");
}

#[test]
fn enum_stored_in_struct() {
    let src = "program E\n\
        enum Status { Aktif, Nonaktif }\n\
        tipe Akun { status: Status\n saldo: i32 }\n\
        utama { nilai a = Akun { status: Status.Aktif, saldo: 10i32 } tampilkan(a.status) tampilkan(a.saldo) }";
    let (out, _) = run_source("instruct", src);
    assert_eq!(out, "Aktif\n10\n");
}

#[test]
fn cocok_dispatches_on_variant() {
    let src = "program E\n\
        enum Warna { Merah, Hijau, Biru }\n\
        fungsi nama(w: Warna) -> teks {\n\
          cocok w {\n\
            Merah => { kembalikan \"merah\" }\n\
            Hijau => { kembalikan \"hijau\" }\n\
            Biru => { kembalikan \"biru\" }\n\
          }\n\
          kembalikan \"?\"\n\
        }\n\
        utama { tampilkan(nama(Warna.Merah)) tampilkan(nama(Warna.Biru)) }";
    let (out, code) = run_source("cocok", src);
    assert_eq!(out, "merah\nbiru\n");
    assert_eq!(code, 0);
}

#[test]
fn cocok_default_and_mutation() {
    let src = "program E\n\
        enum Arah { Naik, Turun, Diam }\n\
        fungsi langkah(a: Arah, p: angka) -> angka {\n\
          ubah q: angka = p\n\
          cocok a {\n\
            Naik => { q = q + 1 }\n\
            lain => { q = q - 1 }\n\
          }\n\
          kembalikan q\n\
        }\n\
        utama { tampilkan(langkah(Arah.Naik, 10)) tampilkan(langkah(Arah.Turun, 10)) tampilkan(langkah(Arah.Diam, 10)) }";
    let (out, _) = run_source("cocok_def", src);
    assert_eq!(out, "11\n9\n9\n");
}
