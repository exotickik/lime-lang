use lime_ast::{Item, Program};
use lime_codegen::{build_executable, emit_llvm_ir, Target};
use lime_diagnostics::{codes, render, Diagnostic, DiagnosticEngine};
use lime_ir::{lower, optimize_level};
use lime_lexer::tokenize;
use lime_parser::parse_with_base;
use lime_sema::{analyze_modules, ModuleInfo, TypeckResults};
use lime_span::SourceMap;
use std::collections::HashSet;
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
        let mut sm = SourceMap::new();
        let mut diags = DiagnosticEngine::new();
        let mut info = ModuleInfo::default();
        let mut items: Vec<Item> = Vec::new();
        let mut entry_name: Option<lime_ast::Ident> = None;
        let mut entry_span = None;

        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let mut loaded: HashSet<String> = HashSet::new();
        let mut queue: Vec<(ModSource, bool)> = vec![(ModSource::File(path.to_path_buf()), true)];
        let mut next_node: u32 = 0;

        while let Some((source, is_entry)) = queue.pop() {
            let key = source.key();
            if !loaded.insert(key.clone()) {
                continue;
            }
            let src = match &source {
                ModSource::File(p) => self.read(p)?,
                ModSource::Std(name) => match std_source(name) {
                    Some(s) => s.to_string(),
                    None => {
                        diags.emit(
                            Diagnostic::error(
                                codes::E_UNSUPPORTED,
                                format!("modul pustaka '{}' tidak ada", name),
                            )
                            .primary(lime_span::Span::point(lime_span::FileId(0), 0), "bukan modul standar"),
                        );
                        continue;
                    }
                },
            };
            let file = sm.add(key, src.clone());
            let toks = tokenize(file, &src, &mut diags);
            let (program, next) = parse_with_base(file, toks, &mut diags, next_node);
            next_node = next;

            let mod_id = info.module_names.len();
            info.module_names.push(program.name.name.clone());
            info.file_module.insert(file, mod_id);
            info.imports.push(HashSet::new());

            if is_entry {
                entry_name = Some(program.name.clone());
                entry_span = Some(program.header_span);
            } else if program.items.iter().any(|i| matches!(i, Item::Main(_))) {
                diags.emit(
                    Diagnostic::error(
                        codes::E_UNSUPPORTED,
                        format!("modul '{}' tidak boleh punya blok 'utama'", program.name.name),
                    )
                    .primary(program.header_span, "hanya file 'program' yang boleh punya 'utama'"),
                );
            }

            for item in &program.items {
                match item {
                    Item::Import(name) => {
                        info.imports[mod_id].insert(name.name.clone());
                        let child = dir.join(format!("{}.lime", name.name));
                        if !child.exists() {
                            diags.emit(
                                Diagnostic::error(
                                    codes::E_UNSUPPORTED,
                                    format!("modul '{}' tidak ditemukan ('{}')", name.name, child.display()),
                                )
                                .primary(name.span, "file modul tidak ada"),
                            );
                        } else {
                            queue.push((ModSource::File(child), false));
                        }
                    }
                    Item::Use(name) => {
                        info.imports[mod_id].insert(name.name.clone());
                        if std_source(&name.name).is_none() {
                            diags.emit(
                                Diagnostic::error(
                                    codes::E_UNSUPPORTED,
                                    format!("modul pustaka '{}' tidak dikenal", name.name),
                                )
                                .primary(name.span, "bukan modul standar")
                                .help("modul standar yang tersedia: Inti, Matematika"),
                            );
                        } else {
                            queue.push((ModSource::Std(name.name.clone()), false));
                        }
                    }
                    _ => {}
                }
            }
            items.extend(program.items);
        }

        let (name, header_span) = match (entry_name, entry_span) {
            (Some(n), Some(s)) => (n, s),
            _ => (
                lime_ast::Ident { name: "program".into(), span: lime_span::Span::point(lime_span::FileId(0), 0) },
                lime_span::Span::point(lime_span::FileId(0), 0),
            ),
        };
        let program = Program { name, header_span, items, is_module: false };
        let results = analyze_modules(&program, &info, &mut diags);
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
        optimize_level(&mut module, self.opt_level);
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
        optimize_level(&mut module, self.opt_level);
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

enum ModSource {
    File(PathBuf),
    Std(String),
}

impl ModSource {
    fn key(&self) -> String {
        match self {
            ModSource::File(p) => p.to_string_lossy().to_string(),
            ModSource::Std(name) => format!("<pustaka>/{}.lime", name),
        }
    }
}

fn std_source(name: &str) -> Option<&'static str> {
    match name {
        "Inti" => Some(include_str!("../../../std/Inti.lime")),
        "Matematika" => Some(include_str!("../../../std/Matematika.lime")),
        _ => None,
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
