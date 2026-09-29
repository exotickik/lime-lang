use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("lime_arr_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn index_read_and_length() {
    let src = "program A\n\
        utama { nilai xs: [i32; 4] = [1i32, 2i32, 3i32, 4i32] tampilkan(xs[0]) tampilkan(xs[3]) tampilkan(panjang(xs)) }";
    let (out, code) = run_source("read", src);
    assert_eq!(out, "1\n4\n4\n");
    assert_eq!(code, 0);
}

#[test]
fn index_mutation() {
    let src = "program A\n\
        utama { ubah xs: [i32; 3] = [5i32, 6i32, 7i32] xs[1] = 60i32 tampilkan(xs[1]) }";
    let (out, _) = run_source("mut", src);
    assert_eq!(out, "60\n");
}

#[test]
fn foreach_iteration_sum() {
    let src = "program A\n\
        utama {\n\
          nilai xs: [angka; 5] = [1, 2, 3, 4, 5]\n\
          ubah total = 0\n\
          untuk x pada xs { total = total + x }\n\
          tampilkan(total)\n\
        }";
    let (out, _) = run_source("foreach", src);
    assert_eq!(out, "15\n");
}

#[test]
fn index_iteration_with_length() {
    let src = "program A\n\
        utama {\n\
          nilai xs: [angka; 3] = [10, 20, 30]\n\
          untuk i dari 0 sampai panjang(xs) { tampilkan(xs[i]) }\n\
        }";
    let (out, _) = run_source("idxiter", src);
    assert_eq!(out, "10\n20\n30\n");
}

#[test]
fn array_as_param_and_return() {
    let src = "program A\n\
        fungsi gandakan(xs: [i32; 3]) -> [i32; 3] {\n\
          kembalikan [xs[0] * 2i32, xs[1] * 2i32, xs[2] * 2i32]\n\
        }\n\
        utama { nilai ys = gandakan([1i32, 2i32, 3i32]) tampilkan(ys[2]) }";
    let (out, _) = run_source("param", src);
    assert_eq!(out, "6\n");
}

#[test]
fn array_of_structs() {
    let src = "program A\n\
        tipe Titik { x: i32\n y: i32 }\n\
        utama {\n\
          nilai ps: [Titik; 2] = [Titik { x: 1i32, y: 2i32 }, Titik { x: 3i32, y: 4i32 }]\n\
          tampilkan(ps[1].y)\n\
        }";
    let (out, _) = run_source("ofstruct", src);
    assert_eq!(out, "4\n");
}

#[test]
fn out_of_bounds_aborts() {
    let src = "program A\n\
        utama { nilai xs: [i32; 3] = [1i32, 2i32, 3i32] ubah i = 9 tampilkan(xs[i]) }";
    let (out, code) = run_source("oob", src);
    assert!(out.is_empty(), "expected no output, got {out:?}");
    assert_ne!(code, 0, "out-of-bounds access must fail");
}
