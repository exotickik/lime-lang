use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_thr_{}_{}", tag, std::process::id()));
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

const WORK: &str = "program P\n\
    fungsi jumlah_sampai(n: angka) -> angka {\n\
        ubah t = 0 ubah i = 0\n\
        selama i < n { t = t + i i = i + 1 }\n\
        kembalikan t\n\
    }\n";

#[test]
fn threads_run_and_join() {
    let src = format!(
        "{WORK}utama {{ nilai a = mulai(jumlah_sampai, 1000) nilai b = mulai(jumlah_sampai, 2000) \
         tampilkan(tunggu(a)) tampilkan(tunggu(b)) }}"
    );
    let (out, code) = run_source("join", &src);
    assert_eq!(out, "499500\n1999000\n");
    assert_eq!(code, 0);
}

#[test]
fn many_threads_sum_correctly() {
    let src = format!(
        "{WORK}utama {{ \
           nilai a = mulai(jumlah_sampai, 100) \
           nilai b = mulai(jumlah_sampai, 200) \
           nilai c = mulai(jumlah_sampai, 300) \
           tampilkan(tunggu(a) + tunggu(b) + tunggu(c)) }}"
    );
    let (out, _) = run_source("many", &src);
    // 4950 + 19900 + 44850
    assert_eq!(out, "69700\n");
}

#[test]
fn mulai_uses_real_threads_in_ir() {
    let src = format!("{WORK}utama {{ nilai a = mulai(jumlah_sampai, 10) tampilkan(tunggu(a)) }}");
    let path = dir("ir").join("prog.exo");
    std::fs::write(&path, &src).unwrap();
    let out = exotickik().arg("emit-ir").arg(&path).output().unwrap();
    let ir = String::from_utf8_lossy(&out.stdout);
    assert!(ir.contains("@pthread_create"), "spawns an OS thread");
    assert!(ir.contains("@pthread_join"), "joins the OS thread");
}

#[test]
fn mulai_requires_angka_worker() {
    let src = "program P\n\
        fungsi salah(x: teks) -> angka { kembalikan 0 }\n\
        utama { nilai a = mulai(salah, 1) }";
    let (code, err) = check_stderr("badfn", src);
    assert_eq!(code, 1);
    assert!(err.contains("E02D0"), "got: {err}");
}

#[test]
fn tunggu_requires_utas() {
    let src = "program P\nutama { nilai x = 5 tampilkan(tunggu(x)) }";
    let (code, err) = check_stderr("badwait", src);
    assert_eq!(code, 1);
    assert!(err.contains("E02D0"), "got: {err}");
}
