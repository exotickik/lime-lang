use lime_ast::{
    Block, Expr, ExprKind, ExternDecl, Ident, IfStmt, Item, MainBlock, NodeId, Param, Program,
    Stmt, TypeExpr, TypeExprKind,
};
use lime_codegen::{build_executable, emit_llvm_ir, Target};
use lime_diagnostics::{codes, render, Diagnostic, DiagnosticEngine};
use lime_ir::{lower, optimize_level};
use lime_lexer::tokenize;
use lime_parser::parse_with_base;
use lime_sema::{analyze_modules, ModuleInfo, TypeckResults};
use lime_span::{FileId, SourceMap, Span};
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

struct Merged {
    sm: SourceMap,
    diags: DiagnosticEngine,
    program: Program,
    info: ModuleInfo,
    next_node: u32,
    entry_file: FileId,
}

impl Compiler {
    fn read(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("tidak bisa membaca '{}': {}", path.display(), e))
    }

    fn parse_merge(&self, path: &Path) -> Result<Merged, String> {
        let mut sm = SourceMap::new();
        let mut diags = DiagnosticEngine::new();
        let mut info = ModuleInfo::default();
        let mut items: Vec<Item> = Vec::new();
        let mut entry_name: Option<lime_ast::Ident> = None;
        let mut entry_span = None;
        let mut entry_file = FileId(0);

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
                            .primary(Span::point(FileId(0), 0), "bukan modul standar"),
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
                entry_file = file;
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
                                .help("modul standar yang tersedia: Inti, Matematika, Teks"),
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
                lime_ast::Ident { name: "program".into(), span: Span::point(FileId(0), 0) },
                Span::point(FileId(0), 0),
            ),
        };
        let program = Program { name, header_span, items, is_module: false };
        Ok(Merged { sm, diags, program, info, next_node, entry_file })
    }

    fn frontend(&self, path: &Path) -> Result<Frontend, String> {
        let mut m = self.parse_merge(path)?;
        let results = analyze_modules(&m.program, &m.info, &mut m.diags);
        Ok(Frontend { sm: m.sm, diags: m.diags, program: m.program, results })
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

    pub fn test(&self, path: &Path) -> i32 {
        let mut m = match self.parse_merge(path) {
            Ok(m) => m,
            Err(msg) => {
                eprintln!("error: {}", msg);
                return 1;
            }
        };
        if m.diags.has_errors() {
            eprint!("{}", render::render_all(m.diags.diagnostics(), &m.sm));
            return 1;
        }

        let tests = collect_tests(&m.program, m.entry_file, &mut m.diags);
        if m.diags.has_errors() {
            eprint!("{}", render::render_all(m.diags.diagnostics(), &m.sm));
            return 1;
        }
        if tests.is_empty() {
            eprintln!("tidak ada fungsi uji (nama 'uji_*') di '{}'", path.display());
            return 1;
        }

        let span = m.program.header_span;
        let has_fflush = m.program.items.iter().any(
            |i| matches!(i, Item::Extern(e) if e.name.name == "fflush"),
        );
        if !has_fflush {
            m.program.items.push(Item::Extern(synth_fflush_decl(&mut m.next_node, span)));
        }
        let main = synth_test_main(&tests, &mut m.next_node, span);
        m.program.items.push(Item::Main(main));

        let results = analyze_modules(&m.program, &m.info, &mut m.diags);
        if m.diags.has_errors() {
            eprint!("{}", render::render_all(m.diags.diagnostics(), &m.sm));
            return 1;
        }
        let mut module = lower(&m.program, &results, &mut m.diags);
        if m.diags.has_errors() {
            eprint!("{}", render::render_all(m.diags.diagnostics(), &m.sm));
            return 1;
        }
        optimize_level(&mut module, self.opt_level);

        let work = std::env::temp_dir().join(format!("lime-test-{}", std::process::id()));
        let out_exe = work.join(exe_stem(path, &self.target));
        let build_work = build_dir(&out_exe);
        if let Err(e) = build_executable(&module, &self.target, &out_exe, &build_work, self.opt_level) {
            eprintln!("error: kegagalan backend: {}", e);
            return 1;
        }

        let output = match std::process::Command::new(&out_exe).output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("error: tidak bisa menjalankan uji: {}", e);
                return 1;
            }
        };
        report_tests(path, &tests, &output)
    }
}

fn collect_tests(program: &Program, entry: FileId, diags: &mut DiagnosticEngine) -> Vec<String> {
    let mut names = Vec::new();
    for item in &program.items {
        let Item::Function(f) = item else { continue };
        if !f.name.name.starts_with("uji_") {
            continue;
        }
        if f.span.file != entry {
            continue;
        }
        let returns_bool = matches!(
            &f.ret,
            Some(t) if matches!(&t.kind, TypeExprKind::Named(n) if n == "boolean")
        );
        if !f.params.is_empty() {
            diags.emit(
                Diagnostic::error(codes::E_UNSUPPORTED, "fungsi uji tidak boleh punya parameter")
                    .primary(f.name.span, "hapus parameter dari fungsi uji ini"),
            );
        } else if !f.type_params.is_empty() {
            diags.emit(
                Diagnostic::error(codes::E_UNSUPPORTED, "fungsi uji tidak boleh generic")
                    .primary(f.name.span, "fungsi uji harus tanpa type parameter"),
            );
        } else if !returns_bool {
            diags.emit(
                Diagnostic::error(codes::E_UNSUPPORTED, "fungsi uji harus mengembalikan boolean")
                    .primary(f.name.span, "ubah tipe kembalian menjadi boolean (benar = lulus)"),
            );
        } else {
            names.push(f.name.name.clone());
        }
    }
    names
}

fn synth_fflush_decl(next: &mut u32, span: Span) -> ExternDecl {
    let angka = || TypeExpr { kind: TypeExprKind::Named("angka".to_string()), span };
    let id = NodeId(*next);
    *next += 1;
    ExternDecl {
        name: Ident { name: "fflush".to_string(), span },
        params: vec![Param {
            name: Ident { name: "aliran".to_string(), span },
            ty: angka(),
            span,
            id,
        }],
        variadic: false,
        ret: Some(angka()),
        span,
    }
}

fn synth_test_main(tests: &[String], next: &mut u32, span: Span) -> MainBlock {
    let mut stmts = Vec::new();
    for name in tests {
        let cond = mk_call(next, span, name, Vec::new());
        let pass = mk_print(next, span, &format!("##LIME-UJI LULUS {}", name));
        let fail = mk_print(next, span, &format!("##LIME-UJI GAGAL {}", name));
        stmts.push(Stmt::If(IfStmt {
            cond,
            then_block: Block { stmts: vec![Stmt::Expr(pass)], span },
            else_ifs: Vec::new(),
            else_block: Some(Block { stmts: vec![Stmt::Expr(fail)], span }),
            span,
        }));
        let zero = mk_expr(next, span, ExprKind::Int { value: 0, suffix: None });
        let flush = mk_call(next, span, "fflush", vec![zero]);
        stmts.push(Stmt::Expr(flush));
    }
    MainBlock { body: Block { stmts, span }, span }
}

fn mk_expr(next: &mut u32, span: Span, kind: ExprKind) -> Expr {
    let id = NodeId(*next);
    *next += 1;
    Expr { id, kind, span }
}

fn mk_call(next: &mut u32, span: Span, callee: &str, args: Vec<Expr>) -> Expr {
    let f = mk_expr(next, span, ExprKind::Name(callee.to_string()));
    mk_expr(next, span, ExprKind::Call { callee: Box::new(f), args })
}

fn mk_print(next: &mut u32, span: Span, text: &str) -> Expr {
    let s = mk_expr(next, span, ExprKind::Str(text.to_string()));
    mk_call(next, span, "tampilkan", vec![s])
}

fn report_tests(path: &Path, tests: &[String], output: &std::process::Output) -> i32 {
    use std::collections::HashMap;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut verdict: HashMap<&str, bool> = HashMap::new();
    let mut extra: Vec<&str> = Vec::new();
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("##LIME-UJI ") {
            if let Some(name) = rest.strip_prefix("LULUS ") {
                verdict.insert(name, true);
            } else if let Some(name) = rest.strip_prefix("GAGAL ") {
                verdict.insert(name, false);
            }
        } else if !line.is_empty() {
            extra.push(line);
        }
    }

    println!("menjalankan {} uji: {}\n", tests.len(), path.display());
    let mut passed = 0usize;
    let mut failed = 0usize;
    for name in tests {
        match verdict.get(name.as_str()) {
            Some(true) => {
                passed += 1;
                println!("  ok     {}", name);
            }
            Some(false) => {
                failed += 1;
                println!("  GAGAL  {}", name);
            }
            None => {
                failed += 1;
                println!("  GAGAL  {} (berhenti sebelum selesai)", name);
            }
        }
    }
    if !extra.is_empty() {
        println!("\nkeluaran program:");
        for line in extra {
            println!("  {}", line);
        }
    }
    println!("\nhasil: {} lulus, {} gagal", passed, failed);
    if failed == 0 && output.status.success() {
        0
    } else {
        1
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
        "Teks" => Some(include_str!("../../../std/Teks.lime")),
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
