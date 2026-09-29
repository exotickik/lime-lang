use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("lime_gen_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn generic_max_multiple_instantiations() {
    let src = "program G\n\
        fungsi maksimum<T>(a: T, b: T) -> T { jika a > b { kembalikan a } kembalikan b }\n\
        utama {\n\
          tampilkan(maksimum(3, 9))\n\
          nilai a: i32 = 40i32\n\
          nilai b: i32 = 12i32\n\
          tampilkan(maksimum(a, b))\n\
          tampilkan(maksimum(2.5, 1.5))\n\
        }";
    let (out, code) = run_source("max", src);
    assert_eq!(out, "9\n40\n2.5\n");
    assert_eq!(code, 0);
}

#[test]
fn generic_identity_across_types() {
    let src = "program G\n\
        fungsi id<T>(x: T) -> T { kembalikan x }\n\
        utama { tampilkan(id(42)) tampilkan(id(\"halo\")) tampilkan(id(benar)) }";
    let (out, _) = run_source("id", src);
    assert_eq!(out, "42\nhalo\nbenar\n");
}

#[test]
fn generic_two_type_params() {
    let src = "program G\n\
        fungsi pertama<A, B>(a: A, b: B) -> A { kembalikan a }\n\
        utama { tampilkan(pertama(7, \"abaikan\")) tampilkan(pertama(\"pilih\", 99)) }";
    let (out, _) = run_source("two", src);
    assert_eq!(out, "7\npilih\n");
}

#[test]
fn generic_calls_generic() {
    let src = "program G\n\
        fungsi id<T>(x: T) -> T { kembalikan x }\n\
        fungsi gandakan<T>(x: T) -> T { kembalikan id(x) }\n\
        utama { tampilkan(gandakan(5)) tampilkan(gandakan(\"a\")) }";
    let (out, _) = run_source("nested", src);
    assert_eq!(out, "5\na\n");
}

#[test]
fn generic_recursion_reuses_instance() {
    let src = "program G\n\
        fungsi jum<T>(n: T, akhir: T) -> T { kembalikan n }\n\
        utama { tampilkan(jum(3, 3)) }";
    let (out, _) = run_source("rec", src);
    assert_eq!(out, "3\n");
}
