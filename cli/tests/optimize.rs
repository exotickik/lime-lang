use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_opt_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(tag: &str, src: &str) -> PathBuf {
    let p = dir(tag).join("prog.lime");
    std::fs::write(&p, src).unwrap();
    p
}

fn run_level(path: &PathBuf, level: &str) -> String {
    let out = lime().arg("run").arg(level).arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const SRC: &str = "program Opt\n\
    utama {\n\
        nilai a = 2 + 3 * 4\n\
        jika benar { tampilkan(a) } lain { tampilkan(0) }\n\
    }\n";

#[test]
fn all_levels_produce_correct_output() {
    let p = write("levels", SRC);
    for lvl in ["-O0", "-O1", "-O2", "-O3"] {
        assert_eq!(run_level(&p, lvl), "14\n", "wrong output at {lvl}");
    }
}

#[test]
fn constants_are_folded_in_ir() {
    let p = write("fold", SRC);
    let out = lime().arg("emit-ir").arg(&p).output().unwrap();
    let ir = String::from_utf8_lossy(&out.stdout);
    assert!(ir.contains("store i64 14"), "2 + 3 * 4 should fold to 14:\n{ir}");
    assert!(!ir.contains("br i1"), "the constant `jika benar` branch should be folded away");
}
