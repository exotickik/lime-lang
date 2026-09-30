use lime_ast::Program;
use lime_codegen::{build_executable, emit_llvm_ir, Target};
use lime_diagnostics::{render, DiagnosticEngine};
use lime_ir::{lower, optimize};
use lime_lexer::tokenize;
use lime_parser::parse;
use lime_sema::{analyze, TypeckResults};
use lime_span::SourceMap;
use std::path::{Path, PathBuf};

pub struct Compiler {
    pub target: Target,
    pub opt_level: u8,
}

impl Default for Compiler {
    fn default() -> Compiler {
        Compiler { target: Target::host(), opt_level: 2 }
    }
}

struct Frontend {
    sm: SourceMap,
    diags: DiagnosticEngine,
    program: Program,
    results: TypeckResults,
}

impl Compiler {
    fn read(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("tidak bisa membaca '{}': {}", path.display(), e))
    }

    fn frontend(&self, path: &Path) -> Result<Frontend, String> {
        let src = self.read(path)?;
        let name = path.to_string_lossy().to_string();
        let mut sm = SourceMap::new();
        let file = sm.add(name, src.clone());
        let mut diags = DiagnosticEngine::new();
        let toks = tokenize(file, &src, &mut diags);
        let program = parse(file, toks, &mut diags);
        let results = analyze(&program, &mut diags);
        Ok(Frontend { sm, diags, program, results })
    }

    pub fn check(&self, path: &Path) -> bool {
        match self.frontend(path) {
            Ok(fe) => {
                eprint!("{}", render::render_all(fe.diags.diagnostics(), &fe.sm));
                if fe.diags.has_errors() {
                    false
                } else {
                    println!("check: '{}' valid", path.display());
                    true
                }
            }
            Err(msg) => {
                eprintln!("error: {}", msg);
                false
            }
        }
    }

    pub fn emit_ir(&self, path: &Path) -> Option<String> {
        let mut fe = self.frontend(path).map_err(|m| eprintln!("error: {m}")).ok()?;
        if fe.diags.has_errors() {
            eprint!("{}", render::render_all(fe.diags.diagnostics(), &fe.sm));
            return None;
        }
        let mut module = lower(&fe.program, &fe.results, &mut fe.diags);
        if fe.diags.has_errors() {
            eprint!("{}", render::render_all(fe.diags.diagnostics(), &fe.sm));
            return None;
        }
        optimize(&mut module);
        Some(emit_llvm_ir(&module, &self.target))
    }

    pub fn build(&self, path: &Path, out: Option<PathBuf>) -> Option<PathBuf> {
        let mut fe = match self.frontend(path) {
            Ok(fe) => fe,
            Err(msg) => {
                eprintln!("error: {}", msg);
                return None;
            }
        };
        if fe.diags.has_errors() {
            eprint!("{}", render::render_all(fe.diags.diagnostics(), &fe.sm));
            return None;
        }
        let mut module = lower(&fe.program, &fe.results, &mut fe.diags);
        if fe.diags.has_errors() {
            eprint!("{}", render::render_all(fe.diags.diagnostics(), &fe.sm));
            return None;
        }
        optimize(&mut module);
        eprint!("{}", render::render_all(fe.diags.diagnostics(), &fe.sm));

        let out_exe = out.unwrap_or_else(|| default_output(path, &self.target));
        let work = build_dir(&out_exe);
        match build_executable(&module, &self.target, &out_exe, &work, self.opt_level) {
            Ok(_) => Some(out_exe),
            Err(e) => {
                eprintln!("error: kegagalan backend: {}", e);
                None
            }
        }
    }

    pub fn run(&self, path: &Path, args: &[String]) -> i32 {
        let work = std::env::temp_dir().join(format!("lime-run-{}", std::process::id()));
        let out_exe = work.join(exe_stem(path, &self.target));
        let Some(exe) = self.build(path, Some(out_exe)) else {
            return 1;
        };
        match std::process::Command::new(&exe).args(args).status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => {
                eprintln!("error: tidak bisa menjalankan '{}': {}", exe.display(), e);
                1
            }
        }
    }
}

fn exe_stem(path: &Path, target: &Target) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "program".to_string());
    format!("{}{}", stem, target.exe_suffix)
}

fn default_output(path: &Path, target: &Target) -> PathBuf {
    PathBuf::from(exe_stem(path, target))
}

fn build_dir(out_exe: &Path) -> PathBuf {
    let parent = out_exe.parent().filter(|p| !p.as_os_str().is_empty());
    match parent {
        Some(p) => p.join(".lime-build"),
        None => PathBuf::from(".lime-build"),
    }
}
