use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn run_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_ffi_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn extern_abs_from_libc() {
    let src = "program F\n\
        ekstern fungsi abs(x: i32) -> i32\n\
        utama { nilai n: i32 = 0i32 - 42i32 tampilkan(abs(n)) }";
    let (out, code) = run_source("abs", src);
    assert_eq!(out, "42\n");
    assert_eq!(code, 0);
}

#[test]
fn extern_putchar_side_effect() {
    let src = "program F\n\
        ekstern fungsi putchar(c: i32) -> i32\n\
        utama { putchar(65i32) putchar(66i32) putchar(10i32) }";
    let (out, _) = run_source("putchar", src);
    assert_eq!(out, "AB\n");
}

#[test]
fn extern_variadic_printf() {
    let src = "program F\n\
        ekstern fungsi printf(fmt: teks, ...) -> i32\n\
        utama { printf(\"x=%d y=%d\\n\", 3i32, 4i32) }";
    let (out, _) = run_source("printf", src);
    assert_eq!(out, "x=3 y=4\n");
}

fn check_source(tag: &str, src: &str) -> (String, i32) {
    let dir = std::env::temp_dir().join(format!("exotickik_ffi_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("prog.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("check").arg(&path).output().unwrap();
    (String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}

#[test]
fn callback_passed_to_libc_atexit() {
    let src = "program F\n\
        ekstern fungsi atexit(f: fungsi()) -> i32\n\
        fungsi pamit() { tampilkan(\"callback\") }\n\
        utama { atexit(pamit) tampilkan(\"utama\") }";
    let (out, code) = run_source("atexit", src);
    assert_eq!(out, "utama\ncallback\n");
    assert_eq!(code, 0);
}

#[test]
fn opaque_pointer_fopen_null_check() {
    let dir = std::env::temp_dir().join(format!("exotickik_ffi_ptr_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let present = dir.join("present.txt");
    std::fs::write(&present, "x").unwrap();
    let present_s = present.to_string_lossy().replace('\\', "/");
    let missing_s = dir.join("nope_zzz.txt").to_string_lossy().replace('\\', "/");
    let src = format!(
        "program P\n\
        ekstern fungsi fopen(nama: teks, mode: teks) -> pointer\n\
        ekstern fungsi fclose(f: pointer) -> i32\n\
        fungsi buka(nama: teks) -> boolean {{\n\
            nilai f = fopen(nama, \"r\")\n\
            jika f == kosong() {{ kembalikan salah }}\n\
            fclose(f)\n\
            kembalikan benar\n\
        }}\n\
        utama {{ tampilkan(buka(\"{present_s}\")) tampilkan(buka(\"{missing_s}\")) }}"
    );
    let (out, code) = run_source("ptr", &src);
    assert_eq!(out, "benar\nsalah\n");
    assert_eq!(code, 0);
}

#[test]
fn pointer_cannot_be_dereferenced() {
    let src = "program P\n\
        ekstern fungsi fopen(nama: teks, mode: teks) -> pointer\n\
        utama { nilai f = fopen(\"x\", \"r\") tampilkan(*f) }";
    let (err, code) = check_source("ptr_deref", src);
    assert_eq!(code, 1);
    assert!(err.contains("'*' hanya untuk reference"), "{err}");
}

#[test]
fn pointer_cannot_be_cast_from_integer() {
    let src = "program P\n\
        utama { nilai p = 5 sebagai pointer tampilkan(1) }";
    let (err, code) = check_source("ptr_cast", src);
    assert_eq!(code, 1);
    assert!(err.contains("menjadi 'pointer'"), "{err}");
}

#[test]
fn reference_still_rejected_across_ffi() {
    let src = "program P\n\
        ekstern fungsi tak_aman(p: &i32) -> i32\n\
        utama { tampilkan(1) }";
    let (err, code) = check_source("ref_ffi", src);
    assert_eq!(code, 1);
    assert!(err.contains("reference Exotickik tidak boleh melewati batas FFI"), "{err}");
}
