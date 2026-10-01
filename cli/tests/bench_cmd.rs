use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_bench_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_bench(tag: &str, src: &str, extra: &[&str]) -> (String, i32) {
    let path = dir(tag).join("tolok.lime");
    std::fs::write(&path, src).unwrap();
    let mut cmd = lime();
    cmd.arg("bench").arg(&path);
    for a in extra {
        cmd.arg(a);
    }
    let out = cmd.output().unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (combined, out.status.code().unwrap_or(-1))
}

#[test]
fn runs_and_reports_ns_per_op() {
    let src = "program P\n\
        fungsi tolok_tambah() -> angka { kembalikan 1 + 1 }\n";
    let (out, code) = run_bench("ok", src, &["--iterasi", "50000"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("tolok_tambah"), "{out}");
    assert!(out.contains("ns/op"), "{out}");
    assert!(out.contains("50000 iterasi"), "{out}");
}

#[test]
fn checksum_proves_work_ran() {
    let src = "program P\n\
        fungsi tolok_tujuh() -> angka { kembalikan 7 }\n";
    let (out, code) = run_bench("sum", src, &["--iterasi", "1000"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("checksum=7000"), "{out}");
}

#[test]
fn wrong_return_type_is_error() {
    let src = "program P\n\
        fungsi tolok_x() -> boolean { kembalikan benar }\n";
    let (out, code) = run_bench("badsig", src, &[]);
    assert_eq!(code, 1);
    assert!(out.contains("tolok ukur harus mengembalikan angka"), "{out}");
}

#[test]
fn no_benches_is_error() {
    let src = "program P\n\
        fungsi biasa() -> angka { kembalikan 1 }\n";
    let (out, code) = run_bench("none", src, &[]);
    assert_eq!(code, 1);
    assert!(out.contains("tidak ada tolok ukur"), "{out}");
}

#[test]
fn abort_preserves_prior_results() {
    let src = "program P\n\
        fungsi tolok_ok() -> angka { kembalikan 1 }\n\
        fungsi tolok_jebol() -> angka {\n\
            nilai a: [angka; 2] = [1, 2]\n\
            ubah i: angka = 7\n\
            kembalikan a[i]\n\
        }\n";
    let (out, code) = run_bench("abort", src, &["--iterasi", "1000"]);
    assert_eq!(code, 1);
    assert!(out.contains("tolok_ok"), "{out}");
    assert!(out.contains("tolok_jebol              berhenti sebelum selesai"), "{out}");
}
