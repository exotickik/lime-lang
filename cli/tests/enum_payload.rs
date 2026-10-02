use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_enump_{}_{}", tag, std::process::id()));
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
fn single_payload_bound_and_used() {
    let src = "program P\n\
        enum Bentuk { Lingkaran(angka), Titik }\n\
        fungsi luas(b: Bentuk) -> angka {\n\
          cocok b { Lingkaran(r) => { kembalikan r * r } Titik => { kembalikan 0 } }\n\
          kembalikan -1\n\
        }\n\
        utama { tampilkan(luas(Bentuk.Lingkaran(6))) tampilkan(luas(Bentuk.Titik)) }";
    let (out, code) = run_source("single", src);
    assert_eq!(out, "36\n0\n");
    assert_eq!(code, 0);
}

#[test]
fn multiple_payload_fields() {
    let src = "program P\n\
        enum Bentuk { Persegi(angka, angka), Titik }\n\
        fungsi luas(b: Bentuk) -> angka {\n\
          cocok b { Persegi(p, l) => { kembalikan p * l } Titik => { kembalikan 0 } }\n\
          kembalikan -1\n\
        }\n\
        utama { tampilkan(luas(Bentuk.Persegi(3, 7))) }";
    let (out, code) = run_source("multi", src);
    assert_eq!(out, "21\n");
    assert_eq!(code, 0);
}

#[test]
fn mixed_payload_and_plain_variants() {
    let src = "program P\n\
        enum Nilai { Bulat(angka), Kebenaran(boolean), Kosong }\n\
        fungsi urai(v: Nilai) -> angka {\n\
          cocok v {\n\
            Bulat(n) => { kembalikan n }\n\
            Kebenaran(b) => { jika b { kembalikan 1 } kembalikan 0 }\n\
            Kosong => { kembalikan -1 }\n\
          }\n\
          kembalikan -2\n\
        }\n\
        utama {\n\
          tampilkan(urai(Nilai.Bulat(99)))\n\
          tampilkan(urai(Nilai.Kebenaran(benar)))\n\
          tampilkan(urai(Nilai.Kosong))\n\
        }";
    let (out, code) = run_source("mixed", src);
    assert_eq!(out, "99\n1\n-1\n");
    assert_eq!(code, 0);
}

#[test]
fn payload_enum_in_daftar() {
    let src = "program P\n\
        enum Token { Angka(angka), Operator(char), Akhir }\n\
        fungsi total(ts: &daftar<Token>) -> angka {\n\
          ubah t = 0\n\
          untuk i dari 0 sampai panjang(ts) {\n\
            cocok elemen(ts, i) { Angka(n) => { t = t + n } lain => {} }\n\
          }\n\
          kembalikan t\n\
        }\n\
        utama {\n\
          ubah ts: daftar<Token> = daftar()\n\
          dorong(ambil_ubah(ts), Token.Angka(10))\n\
          dorong(ambil_ubah(ts), Token.Operator(43 sebagai char))\n\
          dorong(ambil_ubah(ts), Token.Angka(32))\n\
          dorong(ambil_ubah(ts), Token.Akhir)\n\
          tampilkan(total(ambil(ts)))\n\
        }";
    let (out, code) = run_source("daftar", src);
    assert_eq!(out, "42\n");
    assert_eq!(code, 0);
}

#[test]
fn teks_payload_bound() {
    let src = "program P\n\
        enum Pesan { Teks(teks), Diam }\n\
        fungsi tampil(p: Pesan) -> teks {\n\
          cocok p { Teks(s) => { kembalikan s } Diam => { kembalikan \"(diam)\" } }\n\
          kembalikan \"?\"\n\
        }\n\
        utama { tampilkan(tampil(Pesan.Teks(\"halo\"))) tampilkan(tampil(Pesan.Diam)) }";
    let (out, code) = run_source("teks", src);
    assert_eq!(out, "halo\n(diam)\n");
    assert_eq!(code, 0);
}

#[test]
fn nested_enum_payload() {
    let src = "program P\n\
        enum Warna { Merah, Hijau }\n\
        enum Cat { Satu(Warna), Habis }\n\
        fungsi pakai(c: Cat) -> teks {\n\
          cocok c {\n\
            Satu(w) => {\n\
              cocok w { Merah => { kembalikan \"merah\" } Hijau => { kembalikan \"hijau\" } }\n\
            }\n\
            Habis => { kembalikan \"habis\" }\n\
          }\n\
          kembalikan \"?\"\n\
        }\n\
        utama { tampilkan(pakai(Cat.Satu(Warna.Hijau))) tampilkan(pakai(Cat.Habis)) }";
    let (out, code) = run_source("nested", src);
    assert_eq!(out, "hijau\nhabis\n");
    assert_eq!(code, 0);
}

#[test]
fn payload_enum_as_struct_field() {
    let src = "program P\n\
        enum Kejadian { Klik(angka, angka), Tutup }\n\
        tipe Log { terakhir: Kejadian\n jumlah: angka }\n\
        fungsi x(l: Log) -> angka {\n\
          cocok l.terakhir { Klik(a, b) => { kembalikan a + b } Tutup => { kembalikan 0 } }\n\
          kembalikan -1\n\
        }\n\
        utama { nilai l = Log { terakhir: Kejadian.Klik(4, 5), jumlah: 1 } tampilkan(x(l)) }";
    let (out, code) = run_source("structfield", src);
    assert_eq!(out, "9\n");
    assert_eq!(code, 0);
}

#[test]
fn print_payload_variant_name() {
    let src = "program P\n\
        enum Bentuk { Lingkaran(angka), Persegi(angka, angka), Titik }\n\
        utama { tampilkan(Bentuk.Lingkaran(1)) tampilkan(Bentuk.Persegi(1, 2)) tampilkan(Bentuk.Titik) }";
    let (out, code) = run_source("pname", src);
    assert_eq!(out, "Lingkaran\nPersegi\nTitik\n");
    assert_eq!(code, 0);
}

#[test]
fn returned_and_passed_payload_enum() {
    let src = "program P\n\
        enum Opsi { Ada(angka), Tidak }\n\
        fungsi bungkus(n: angka) -> Opsi { jika n > 0 { kembalikan Opsi.Ada(n) } kembalikan Opsi.Tidak }\n\
        fungsi buka(o: Opsi) -> angka { cocok o { Ada(n) => { kembalikan n } Tidak => { kembalikan 0 } } kembalikan -1 }\n\
        utama { tampilkan(buka(bungkus(7))) tampilkan(buka(bungkus(-3))) }";
    let (out, code) = run_source("roundtrip", src);
    assert_eq!(out, "7\n0\n");
    assert_eq!(code, 0);
}

#[test]
fn match_ignoring_payload_still_dispatches() {
    let src = "program P\n\
        enum Bentuk { Lingkaran(angka), Titik }\n\
        fungsi ada_isi(b: Bentuk) -> boolean {\n\
          cocok b { Lingkaran => { kembalikan benar } Titik => { kembalikan salah } }\n\
          kembalikan salah\n\
        }\n\
        utama { tampilkan(ada_isi(Bentuk.Lingkaran(5))) tampilkan(ada_isi(Bentuk.Titik)) }";
    let (out, code) = run_source("ignorepayload", src);
    assert_eq!(out, "benar\nsalah\n");
    assert_eq!(code, 0);
}

#[test]
fn wrong_construction_arity_rejected() {
    let src = "program E\n\
        enum B { Lingkaran(angka), Titik }\n\
        utama { nilai x: B = B.Lingkaran(1, 2) }";
    let (code, err) = check_stderr("arity", src);
    assert_ne!(code, 0);
    assert!(err.contains("E0220"), "stderr: {err}");
}

#[test]
fn binding_count_mismatch_rejected() {
    let src = "program E\n\
        enum B { Lingkaran(angka), Titik }\n\
        fungsi f(b: B) -> angka { cocok b { Lingkaran(a, c) => { kembalikan a } Titik => { kembalikan 0 } } kembalikan 0 }\n\
        utama { tampilkan(f(B.Titik)) }";
    let (code, err) = check_stderr("bindcount", src);
    assert_ne!(code, 0);
    assert!(err.contains("pengikat"), "stderr: {err}");
}

#[test]
fn binding_on_plain_variant_rejected() {
    let src = "program E\n\
        enum B { Lingkaran(angka), Titik }\n\
        fungsi f(b: B) -> angka { cocok b { Lingkaran(a) => { kembalikan a } Titik(z) => { kembalikan 0 } } kembalikan 0 }\n\
        utama { tampilkan(f(B.Titik)) }";
    let (code, err) = check_stderr("plainbind", src);
    assert_ne!(code, 0);
    assert!(err.contains("tidak membawa data"), "stderr: {err}");
}

#[test]
fn bare_payload_variant_rejected() {
    let src = "program E\n\
        enum B { Lingkaran(angka), Titik }\n\
        utama { nilai x: B = B.Lingkaran }";
    let (code, err) = check_stderr("bare", src);
    assert_ne!(code, 0);
    assert!(err.contains("E0220"), "stderr: {err}");
}

#[test]
fn owned_payload_rejected() {
    let src = "program E\n\
        enum B { Buf(daftar<bita>), Titik }\n\
        utama { nilai x: B = B.Titik }";
    let (code, err) = check_stderr("owned", src);
    assert_ne!(code, 0);
    assert!(err.contains("milik"), "stderr: {err}");
}

#[test]
fn equality_on_payload_enum_rejected() {
    let src = "program E\n\
        enum B { Lingkaran(angka), Titik }\n\
        utama { nilai a: B = B.Titik nilai c = a == B.Titik tampilkan(c) }";
    let (code, err) = check_stderr("eq", src);
    assert_ne!(code, 0);
    assert!(err.contains("cocok"), "stderr: {err}");
}

#[test]
fn construction_arg_type_mismatch_rejected() {
    let src = "program E\n\
        enum B { Lingkaran(angka), Titik }\n\
        utama { nilai x: B = B.Lingkaran(benar) }";
    let (code, err) = check_stderr("argty", src);
    assert_ne!(code, 0);
    assert!(err.contains("E0212"), "stderr: {err}");
}
