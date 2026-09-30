use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn project(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_mod_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (name, src) in files {
        std::fs::write(d.join(name), src).unwrap();
    }
    d.join("main.lime")
}

fn run(tag: &str, files: &[(&str, &str)]) -> (String, i32) {
    let entry = project(tag, files);
    let out = lime().arg("run").arg(&entry).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn check(tag: &str, files: &[(&str, &str)]) -> (i32, String) {
    let entry = project(tag, files);
    let out = lime().arg("check").arg(&entry).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

const MATH: (&str, &str) = (
    "Matematika.lime",
    "modul Matematika\n\
     publik fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
     publik fungsi kuadrat(n: angka) -> angka { kembalikan n * n }\n\
     fungsi rahasia(n: angka) -> angka { kembalikan n - 1 }\n",
);

#[test]
fn cross_module_function_call() {
    let entry = (
        "main.lime",
        "program P\nimpor Matematika\nutama { tampilkan(tambah(3, 4)) tampilkan(kuadrat(5)) }",
    );
    let (out, code) = run("call", &[MATH, entry]);
    assert_eq!(out, "7\n25\n");
    assert_eq!(code, 0);
}

#[test]
fn cross_module_struct_and_trait() {
    let geo = (
        "Geometri.lime",
        "modul Geometri\n\
         publik tipe Titik { x: angka, y: angka }\n\
         publik sifat Ukur { fungsi ukur(diri: &Diri) -> angka }\n\
         implementasi Ukur untuk Titik { fungsi ukur(diri: &Diri) -> angka { kembalikan diri.x + diri.y } }\n",
    );
    let entry = (
        "main.lime",
        "program P\nimpor Geometri\nutama { nilai t = Titik { x: 3, y: 4 } tampilkan(t.ukur()) }",
    );
    let (out, _) = run("struct", &[geo, entry]);
    assert_eq!(out, "7\n");
}

#[test]
fn private_item_is_inaccessible() {
    let entry = ("main.lime", "program P\nimpor Matematika\nutama { tampilkan(rahasia(5)) }");
    let (code, err) = check("private", &[MATH, entry]);
    assert_eq!(code, 1);
    assert!(err.contains("E02C0"), "got: {err}");
}

#[test]
fn public_item_needs_import() {
    let user = (
        "Pengguna.lime",
        "modul Pengguna\npublik fungsi pakai() -> angka { kembalikan tambah(1, 2) }\n",
    );
    let entry =
        ("main.lime", "program P\nimpor Matematika\nimpor Pengguna\nutama { tampilkan(pakai()) }");
    let (code, err) = check("noimport", &[MATH, user, entry]);
    assert_eq!(code, 1);
    assert!(err.contains("E02C0"), "got: {err}");
}

#[test]
fn cyclic_imports_terminate() {
    let a = ("A.lime", "modul A\nimpor B\npublik fungsi fa() -> angka { kembalikan 1 }\n");
    let b = ("B.lime", "modul B\nimpor A\npublik fungsi fb() -> angka { kembalikan 2 }\n");
    let entry = ("main.lime", "program P\nimpor A\nimpor B\nutama { tampilkan(fa() + fb()) }");
    let (out, code) = run("cycle", &[a, b, entry]);
    assert_eq!(out, "3\n");
    assert_eq!(code, 0);
}

#[test]
fn missing_module_is_error() {
    let entry = ("main.lime", "program P\nimpor TidakAda\nutama { }");
    let (code, err) = check("missing", &[entry]);
    assert_eq!(code, 1);
    assert!(err.contains("tidak ditemukan"), "got: {err}");
}
