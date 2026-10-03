use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn compiler(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("compiler").join(rel)
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_shstmt_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// Run a statement body (ending in `kembalikan`) through the Rust bootstrap by
// wrapping it in a function and printing its result.
fn bootstrap_run(body: &str) -> i64 {
    let src = format!("program P\nfungsi f() -> angka {{ {body} }}\nutama {{ tampilkan(f()) }}\n");
    let path = dir().join("p.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().unwrap()
}

#[test]
fn self_hosted_statement_parser_passes_its_own_suite() {
    let out = exotickik().arg("test").arg(compiler("uji_stmt.exo")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("12 lulus, 0 gagal"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn bootstrap_agrees_on_statement_programs() {
    // The self-hosted statement parser + evaluator (compiler/uji_stmt.exo)
    // asserts these bodies evaluate to these values; the Rust bootstrap runs the
    // same bodies as a function, establishing head-to-head agreement on
    // let/ubah/assign, identifier resolution, and evaluation order.
    let cases: &[(&str, i64)] = &[
        ("nilai x = 10 kembalikan x + 5", 15),
        ("nilai x = 2 nilai y = 3 kembalikan x * y + 1", 7),
        ("ubah a = 1 a = a + 4 kembalikan a", 5),
        ("ubah x = 10 nilai y = x * 2 x = 1 kembalikan x + y", 21),
        ("nilai sama = 5 nilai jumlah = sama + sama kembalikan jumlah", 10),
        ("nilai a = 2 nilai b = 3 nilai c = 4 kembalikan a + b * c", 14),
        ("ubah x = 0 jika 3 > 2 { x = 10 } lain { x = 20 } kembalikan x", 10),
        ("nilai n = 2 ubah x = 0 jika n == 1 { x = 10 } lain jika n == 2 { x = 20 } lain { x = 30 } kembalikan x", 20),
        ("ubah i = 0 ubah jumlah = 0 selama i < 5 { jumlah = jumlah + i i = i + 1 } kembalikan jumlah", 10),
        ("ubah i = 0 selama i < 100 { jika i == 7 { kembalikan i } i = i + 1 } kembalikan 0 - 1", 7),
        ("ubah total = 0 ubah i = 1 selama i <= 3 { ubah j = 1 selama j <= 3 { total = total + i * j j = j + 1 } i = i + 1 } kembalikan total", 36),
    ];
    for (body, expected) in cases {
        assert_eq!(bootstrap_run(body), *expected, "body: {body}");
    }
}
