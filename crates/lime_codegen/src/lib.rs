pub mod emit;
pub mod target;

pub use emit::emit_module;
pub use target::Target;

use lime_ir::Module;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum CodegenError {
    ToolNotFound(String),
    ToolFailed { tool: String, stderr: String },
    Io(std::io::Error),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::ToolNotFound(t) => {
                write!(f, "alat '{}' tidak ditemukan pada PATH", t)
            }
            CodegenError::ToolFailed { tool, stderr } => {
                write!(f, "alat '{}' gagal:\n{}", tool, stderr)
            }
            CodegenError::Io(e) => write!(f, "kesalahan IO: {}", e),
        }
    }
}

impl From<std::io::Error> for CodegenError {
    fn from(e: std::io::Error) -> Self {
        CodegenError::Io(e)
    }
}

pub fn emit_llvm_ir(module: &Module, target: &Target) -> String {
    emit_module(module, target)
}

pub struct BuildArtifacts {
    pub ll_path: PathBuf,
    pub obj_path: PathBuf,
    pub exe_path: PathBuf,
}

pub fn build_executable(
    module: &Module,
    target: &Target,
    out_exe: &Path,
    work_dir: &Path,
    opt_level: u8,
) -> Result<BuildArtifacts, CodegenError> {
    std::fs::create_dir_all(work_dir)?;
    let stem = if module.name.is_empty() { "program" } else { module.name.as_str() };

    let ll_path = work_dir.join(format!("{stem}.ll"));
    let obj_path = work_dir.join(format!("{stem}.o"));
    let ir = emit_module(module, target);
    std::fs::write(&ll_path, ir)?;

    compile_ir_to_object(&ll_path, &obj_path, opt_level)?;
    link_object(&obj_path, out_exe, target)?;

    Ok(BuildArtifacts { ll_path, obj_path, exe_path: out_exe.to_path_buf() })
}

fn compile_ir_to_object(ll: &Path, obj: &Path, opt_level: u8) -> Result<(), CodegenError> {
    let llc = find_tool(&["LIME_LLC"], &["llc", "llc-18", "llc-17"])?;
    let opt = format!("-O{}", opt_level.min(3));
    let output = Command::new(&llc)
        .arg("-filetype=obj")
        .arg("-relocation-model=pic")
        .arg(&opt)
        .arg(ll)
        .arg("-o")
        .arg(obj)
        .output()?;
    if !output.status.success() {
        return Err(CodegenError::ToolFailed {
            tool: llc,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

fn link_object(obj: &Path, out_exe: &Path, target: &Target) -> Result<(), CodegenError> {
    if target.linker_is_cc {
        let cc = find_tool(&["LIME_CC"], &[target.linker, "cc", "clang", "gcc"])?;
        let output = Command::new(&cc).arg(obj).arg("-o").arg(out_exe).output()?;
        if !output.status.success() {
            return Err(CodegenError::ToolFailed {
                tool: cc,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    } else {
        let linker = find_tool(&["LIME_LINKER"], &[target.linker])?;
        let output = Command::new(&linker)
            .arg(format!("/out:{}", out_exe.display()))
            .arg(obj)
            .output()?;
        if !output.status.success() {
            return Err(CodegenError::ToolFailed {
                tool: linker,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    }
}

fn find_tool(env_vars: &[&str], candidates: &[&str]) -> Result<String, CodegenError> {
    for var in env_vars {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return Ok(val);
            }
        }
    }
    for cand in candidates {
        if Command::new(cand).arg("--version").output().is_ok() {
            return Ok(cand.to_string());
        }
    }
    Err(CodegenError::ToolNotFound(candidates.first().copied().unwrap_or("?").to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lime_diagnostics::DiagnosticEngine;
    use lime_lexer::tokenize;
    use lime_parser::parse;
    use lime_span::SourceMap;

    fn compile_and_run(src: &str, tag: &str) -> String {
        let mut sm = SourceMap::new();
        let f = sm.add("t.lime", src);
        let mut diags = DiagnosticEngine::new();
        let toks = tokenize(f, src, &mut diags);
        let prog = parse(f, toks, &mut diags);
        let results = lime_sema::analyze(&prog, &mut diags);
        assert!(!diags.has_errors(), "frontend errors: {:?}", diags.diagnostics());
        let mut module = lime_ir::lower(&prog, &results, &mut diags);
        assert!(!diags.has_errors(), "lowering errors: {:?}", diags.diagnostics());
        lime_ir::optimize(&mut module);

        let target = Target::host();
        let work = std::env::temp_dir().join(format!("lime_cgtest_{}_{}", tag, std::process::id()));
        let exe = work.join("out");
        build_executable(&module, &target, &exe, &work, 2).expect("build failed");
        let output = std::process::Command::new(&exe).output().expect("run failed");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[test]
    fn hello_world_runs() {
        let out = compile_and_run("program Halo\nutama { tampilkan(\"Halo dunia!\") }", "hello");
        assert_eq!(out, "Halo dunia!\n");
    }

    #[test]
    fn arithmetic_and_function() {
        let src = "program P\n\
            fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
            utama { tampilkan(tambah(20, 22)) }";
        let out = compile_and_run(src, "arith");
        assert_eq!(out, "42\n");
    }

    #[test]
    fn loop_and_conditional() {
        let src = "program P\n\
            utama {\n\
              ubah jumlah = 0\n\
              untuk i dari 1 hingga 5 { jumlah = jumlah + i }\n\
              tampilkan(jumlah)\n\
            }";
        let out = compile_and_run(src, "loop");
        assert_eq!(out, "15\n");
    }
}
