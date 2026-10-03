use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn compiler(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("compiler").join(rel)
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_shfn_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// Run function definitions through the Rust bootstrap by adding a utama that
// prints the entry function's result.
fn bootstrap_run(defs: &str, entry: &str) -> i64 {
    let src = format!("program P\n{defs}\nutama {{ tampilkan({entry}()) }}\n");
    let path = dir().join("p.exo");
    std::fs::write(&path, src).unwrap();
    let out = exotickik().arg("run").arg(&path).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().unwrap()
}

#[test]
fn self_hosted_function_parser_passes_its_own_suite() {
    let out = exotickik().arg("test").arg(compiler("uji_fungsi.exo")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("6 lulus, 0 gagal"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn bootstrap_agrees_on_function_programs() {
    // The self-hosted item parser + evaluator (compiler/uji_fungsi.exo) asserts
    // these programs evaluate to these values; the Rust bootstrap runs the same
    // definitions, establishing agreement on function declarations, calls,
    // recursion and parameter passing.
    let cases: &[(&str, &str, i64)] = &[
        ("fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b } fungsi mula() -> angka { kembalikan tambah(2, 3) }", "mula", 5),
        ("fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b } fungsi mula() -> angka { kembalikan tambah(tambah(1, 2), tambah(3, 4)) }", "mula", 10),
        ("fungsi fib(n: angka) -> angka { jika n < 2 { kembalikan n } kembalikan fib(n - 1) + fib(n - 2) } fungsi mula() -> angka { kembalikan fib(10) }", "mula", 55),
        ("fungsi fak(n: angka) -> angka { ubah hasil = 1 ubah i = 1 selama i <= n { hasil = hasil * i i = i + 1 } kembalikan hasil } fungsi mula() -> angka { kembalikan fak(5) }", "mula", 120),
        ("fungsi kuadrat(x: angka) -> angka { kembalikan x * x } fungsi jumlah_kuadrat(a: angka, b: angka) -> angka { kembalikan kuadrat(a) + kuadrat(b) } fungsi mula() -> angka { kembalikan jumlah_kuadrat(3, 4) }", "mula", 25),
        ("publik fungsi dua_kali(x: angka) -> angka { kembalikan x + x } fungsi mula() -> angka { kembalikan dua_kali(21) }", "mula", 42),
    ];
    for (defs, entry, expected) in cases {
        assert_eq!(bootstrap_run(defs, entry), *expected, "defs: {defs}");
    }
}
