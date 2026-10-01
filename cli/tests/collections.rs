use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_col_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn emit_ir(tag: &str, src: &str) -> String {
    let path = dir(tag).join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("emit-ir").arg(&path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn grow_push_get_set_len() {
    let src = "program P\nutama {\n\
        ubah d: daftar<angka> = daftar()\n\
        ubah i = 0\n\
        selama i < 10 { dorong(ambil_ubah(d), i * i) i = i + 1 }\n\
        tampilkan(panjang(ambil(d)))\n\
        tampilkan(elemen(ambil(d), 0))\n\
        tampilkan(elemen(ambil(d), 9))\n\
        atur(ambil_ubah(d), 5, 999)\n\
        tampilkan(elemen(ambil(d), 5))\n\
    }";
    let (out, code) = run_source("grow", src);
    assert_eq!(out, "10\n0\n81\n999\n");
    assert_eq!(code, 0);
}

#[test]
fn list_of_strings() {
    let src = "program P\nutama {\n\
        ubah d: daftar<teks> = daftar()\n\
        dorong(ambil_ubah(d), \"halo\")\n\
        dorong(ambil_ubah(d), \"dunia\")\n\
        tampilkan(elemen(ambil(d), 0))\n\
        tampilkan(elemen(ambil(d), 1))\n\
        tampilkan(panjang(ambil(d)))\n\
    }";
    let (out, _) = run_source("str", src);
    assert_eq!(out, "halo\ndunia\n2\n");
}

#[test]
fn out_of_bounds_aborts() {
    let src = "program P\nutama {\n\
        ubah d: daftar<angka> = daftar() dorong(ambil_ubah(d), 1)\n\
        tampilkan(elemen(ambil(d), 5))\n\
    }";
    let (_, code) = run_source("oob", src);
    assert_ne!(code, 0, "out-of-bounds access must abort");
}

#[test]
fn buffer_is_freed_on_drop() {
    let src = "program P\nutama {\n\
        ubah d: daftar<angka> = daftar() dorong(ambil_ubah(d), 1) tampilkan(panjang(ambil(d)))\n\
    }";
    let ir = emit_ir("free", src);
    assert_eq!(ir.matches("call void @free").count(), 1, "the heap buffer must be freed once");
}

#[test]
fn list_in_struct_and_passed_around() {
    let src = "program P\n\
        fungsi total(d: &daftar<angka>) -> angka {\n\
            ubah s = 0 ubah i = 0\n\
            selama i < panjang(d) { s = s + elemen(d, i) i = i + 1 }\n\
            kembalikan s\n\
        }\n\
        utama {\n\
            ubah d: daftar<angka> = daftar()\n\
            dorong(ambil_ubah(d), 10) dorong(ambil_ubah(d), 20) dorong(ambil_ubah(d), 30)\n\
            tampilkan(total(ambil(d)))\n\
        }";
    let (out, _) = run_source("pass", src);
    assert_eq!(out, "60\n");
}
