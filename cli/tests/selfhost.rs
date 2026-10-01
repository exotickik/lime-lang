use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn example(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("examples").join(rel)
}

#[test]
fn lime_written_calculator_evaluates_correctly() {
    let out = lime().arg("run").arg(example("kalkulator/main.lime")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "182\n230\n98\n56\n100\n"
    );
}

#[test]
fn lime_written_calculator_passes_its_own_tests() {
    let out = lime().arg("test").arg(example("kalkulator/uji.lime")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("6 lulus, 0 gagal"));
}

#[test]
fn lime_written_ast_calculator_evaluates_correctly() {
    let out = lime().arg("run").arg(example("pohon/main.lime")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "182\n230\n98\n56\n100\n"
    );
}

#[test]
fn lime_written_ast_calculator_passes_its_own_tests() {
    let out = lime().arg("test").arg(example("pohon/uji.lime")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("6 lulus, 0 gagal"));
}
