use std::path::PathBuf;
use std::process::Command;

fn lime() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lime"))
}

fn proj(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lime_proj_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn init_then_run_in_project_mode() {
    let dir = proj("init");
    let out = lime().arg("init").arg("demo").current_dir(&dir).output().unwrap();
    assert!(out.status.success());
    assert!(dir.join("lime.toml").is_file());
    assert!(dir.join("src/main.lime").is_file());

    let run = lime().arg("run").current_dir(&dir).output().unwrap();
    assert_eq!(run.status.code().unwrap_or(-1), 0);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "Halo dari Lime!\n");
}

#[test]
fn custom_entry_is_resolved() {
    let dir = proj("entry");
    std::fs::create_dir_all(dir.join("kode")).unwrap();
    std::fs::write(
        dir.join("lime.toml"),
        "[paket]\nnama = \"c\"\n[build]\nentri = \"kode/mulai.lime\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("kode/mulai.lime"),
        "program C\nutama { tampilkan(\"kustom\") }\n",
    )
    .unwrap();
    let run = lime().arg("run").current_dir(&dir).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), "kustom\n");
}

#[test]
fn found_from_subdirectory() {
    let dir = proj("ancestor");
    let sub = dir.join("a/b");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(dir.join("lime.toml"), "[paket]\nnama = \"c\"\n").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.lime"), "program C\nutama { tampilkan(\"dalam\") }\n").unwrap();
    let run = lime().arg("run").current_dir(&sub).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), "dalam\n");
}

#[test]
fn no_file_and_no_manifest_errors() {
    let dir = proj("empty");
    let run = lime().arg("run").current_dir(&dir).output().unwrap();
    assert_eq!(run.status.code().unwrap_or(-1), 1);
    assert!(String::from_utf8_lossy(&run.stderr).contains("lime.toml"));
}

#[test]
fn missing_entry_errors_clearly() {
    let dir = proj("noentry");
    std::fs::write(
        dir.join("lime.toml"),
        "[paket]\nnama = \"c\"\n[build]\nentri = \"tidak/ada.lime\"\n",
    )
    .unwrap();
    let run = lime().arg("run").current_dir(&dir).output().unwrap();
    assert_eq!(run.status.code().unwrap_or(-1), 1);
    assert!(String::from_utf8_lossy(&run.stderr).contains("tidak ada"));
}

#[test]
fn explicit_file_overrides_project_mode() {
    let dir = proj("explicit");
    std::fs::write(dir.join("lime.toml"), "[paket]\nnama = \"c\"\n").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.lime"), "program C\nutama { tampilkan(\"proyek\") }\n").unwrap();
    let other = dir.join("lain.lime");
    std::fs::write(&other, "program L\nutama { tampilkan(\"eksplisit\") }\n").unwrap();
    let run = lime().arg("run").arg(&other).current_dir(&dir).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), "eksplisit\n");
}
