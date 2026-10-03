use std::path::PathBuf;
use std::process::Command;

fn exotickik() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exotickik"))
}

fn compiler(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("compiler").join(rel)
}

#[test]
fn self_hosted_interner_passes_its_own_suite() {
    let out = exotickik().arg("test").arg(compiler("uji_intern.exo")).output().unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("6 lulus, 0 gagal"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
