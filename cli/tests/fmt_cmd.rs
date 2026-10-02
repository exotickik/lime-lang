use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("exotickik_fmt_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

const MESSY: &str = "program X\nutama {\ntampilkan(  1  )\n}\n";

#[test]
fn check_flags_unformatted_file() {
    let d = dir("check_dirty");
    let f = d.join("a.exo");
    std::fs::write(&f, MESSY).unwrap();
    let out = exotickik().arg("fmt").arg("--check").arg(&f).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 1);
    assert!(String::from_utf8_lossy(&out.stdout).contains("perlu format"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), MESSY, "check must not modify");
}

#[test]
fn check_passes_formatted_file() {
    let d = dir("check_clean");
    let f = d.join("a.exo");
    std::fs::write(&f, MESSY).unwrap();
    assert!(exotickik().arg("fmt").arg("--write").arg(&f).output().unwrap().status.success());
    let out = exotickik().arg("fmt").arg("--check").arg(&f).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0);
}

#[test]
fn write_is_idempotent() {
    let d = dir("idem");
    let f = d.join("a.exo");
    std::fs::write(&f, MESSY).unwrap();
    exotickik().arg("fmt").arg("--write").arg(&f).output().unwrap();
    let once = std::fs::read_to_string(&f).unwrap();
    exotickik().arg("fmt").arg("--write").arg(&f).output().unwrap();
    let twice = std::fs::read_to_string(&f).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn formats_a_directory_recursively() {
    let d = dir("recurse");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    std::fs::write(d.join("a.exo"), MESSY).unwrap();
    std::fs::write(d.join("sub/b.exo"), MESSY).unwrap();
    let out = exotickik().arg("fmt").arg("--check").arg(&d).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 1);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("a.exo"), "{text}");
    assert!(text.contains("b.exo"), "{text}");
}

#[test]
fn single_file_prints_to_stdout() {
    let d = dir("stdout");
    let f = d.join("a.exo");
    std::fs::write(&f, MESSY).unwrap();
    let out = exotickik().arg("fmt").arg(&f).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("tampilkan(1)"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), MESSY, "stdout mode must not modify");
}

#[test]
fn multiple_files_without_mode_errors() {
    let d = dir("multi");
    std::fs::write(d.join("a.exo"), MESSY).unwrap();
    std::fs::write(d.join("b.exo"), MESSY).unwrap();
    let out = exotickik()
        .arg("fmt")
        .arg(d.join("a.exo"))
        .arg(d.join("b.exo"))
        .output()
        .unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--write"));
}

#[test]
fn project_mode_formats_manifest_dir() {
    let d = dir("project");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("exotickik.toml"), "[paket]\nnama = \"p\"\n").unwrap();
    std::fs::write(d.join("src/main.exo"), MESSY).unwrap();
    let out = exotickik().arg("fmt").arg("--check").current_dir(&d).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 1);
    assert!(String::from_utf8_lossy(&out.stdout).contains("main.exo"));
}
