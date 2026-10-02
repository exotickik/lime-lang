use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn compiler(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("compiler").join(rel)
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_shparser_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// Evaluate an integer expression through the Rust bootstrap by compiling and
// running `utama { tampilkan(<expr>) }`, returning the printed value.
fn bootstrap_eval(expr: &str) -> i64 {
    let src = format!("program P\nutama {{ tampilkan({expr}) }}\n");
    let path = dir().join("p.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().unwrap()
}

#[test]
fn self_hosted_parser_passes_its_own_suite() {
    let out = exotickik().arg("test").arg(compiler("uji_parser.exo")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("8 lulus, 0 gagal"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn bootstrap_agrees_on_arithmetic_expressions() {
    // The self-hosted parser+evaluator (compiler/uji_parser.exo) asserts these
    // same expressions evaluate to these values; here the Rust bootstrap
    // evaluates each, establishing head-to-head agreement on precedence and
    // associativity.
    let cases: &[(&str, i64)] = &[
        ("12 + 34 * 5", 182),
        ("(12 + 34) * 5", 230),
        ("100 - 2 * 3 + 4", 98),
        ("20 / 5 / 2", 2),
        ("10 - 3 - 2", 5),
        ("2 * 3 + 4 * 5", 26),
        ("7 % 3", 1),
        ("0 - 5 + 3", -2),
    ];
    for (expr, expected) in cases {
        assert_eq!(bootstrap_eval(expr), *expected, "expr: {expr}");
    }
}
