use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_test_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_test(tag: &str, src: &str) -> (String, i32) {
    let path = dir(tag).join("uji.lime");
    std::fs::write(&path, src).unwrap();
    let out = lime().arg("test").arg(&path).output().unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (combined, out.status.code().unwrap_or(-1))
}

#[test]
fn all_tests_pass_exit_zero() {
    let src = "program P\n\
        fungsi uji_a() -> boolean { kembalikan benar }\n\
        fungsi uji_b() -> boolean { kembalikan 2 > 1 }\n";
    let (out, code) = run_test("pass", src);
    assert_eq!(code, 0);
    assert!(out.contains("ok     uji_a"), "{out}");
    assert!(out.contains("ok     uji_b"), "{out}");
    assert!(out.contains("hasil: 2 lulus, 0 gagal"), "{out}");
}

#[test]
fn failing_test_exit_one() {
    let src = "program P\n\
        fungsi uji_ok() -> boolean { kembalikan benar }\n\
        fungsi uji_salah() -> boolean { kembalikan 1 == 2 }\n";
    let (out, code) = run_test("fail", src);
    assert_eq!(code, 1);
    assert!(out.contains("ok     uji_ok"), "{out}");
    assert!(out.contains("GAGAL  uji_salah"), "{out}");
    assert!(out.contains("hasil: 1 lulus, 1 gagal"), "{out}");
}

#[test]
fn wrong_return_type_is_error() {
    let src = "program P\n\
        fungsi uji_x() -> angka { kembalikan 1 }\n";
    let (out, code) = run_test("badsig", src);
    assert_eq!(code, 1);
    assert!(out.contains("fungsi uji harus mengembalikan boolean"), "{out}");
}

#[test]
fn test_with_parameters_is_error() {
    let src = "program P\n\
        fungsi uji_x(n: angka) -> boolean { kembalikan n > 0 }\n";
    let (out, code) = run_test("param", src);
    assert_eq!(code, 1);
    assert!(out.contains("fungsi uji tidak boleh punya parameter"), "{out}");
}

#[test]
fn no_tests_is_error() {
    let src = "program P\n\
        fungsi biasa() -> boolean { kembalikan benar }\n";
    let (out, code) = run_test("none", src);
    assert_eq!(code, 1);
    assert!(out.contains("tidak ada fungsi uji"), "{out}");
}

#[test]
fn runtime_abort_preserves_prior_results() {
    let src = "program P\n\
        fungsi uji_pertama() -> boolean { kembalikan benar }\n\
        fungsi uji_jebol() -> boolean {\n\
            nilai a: [angka; 2] = [1, 2]\n\
            ubah i: angka = 9\n\
            kembalikan a[i] == 0\n\
        }\n\
        fungsi uji_ketiga() -> boolean { kembalikan benar }\n";
    let (out, code) = run_test("abort", src);
    assert_eq!(code, 1);
    assert!(out.contains("ok     uji_pertama"), "{out}");
    assert!(out.contains("GAGAL  uji_jebol (berhenti sebelum selesai)"), "{out}");
}
