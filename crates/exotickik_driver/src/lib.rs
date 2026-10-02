use exotickik_ast::{
    BinOp, Block, Expr, ExprKind, ExternDecl, Ident, IfStmt, Item, MainBlock, NodeId, Param,
    Program, Stmt, TypeExpr, TypeExprKind,
};
use exotickik_codegen::{build_executable, emit_llvm_ir, Target};
use exotickik_diagnostics::{codes, render, Diagnostic, DiagnosticEngine};
use exotickik_ir::{lower, optimize_level};
use exotickik_lexer::tokenize;
use exotickik_parser::parse_with_base;
use exotickik_sema::{analyze_modules, ModuleInfo, TypeckResults};
use exotickik_span::{FileId, SourceMap, Span};
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
        let mut entry_name: Option<exotickik_ast::Ident> = None;
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
                        let child = dir.join(format!("{}.exo", name.name));
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
                exotickik_ast::Ident { name: "program".into(), span: Span::point(FileId(0), 0) },
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
        let work = std::env::temp_dir().join(format!("exotickik-run-{}", std::process::id()));
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

        let work = std::env::temp_dir().join(format!("exotickik-test-{}", std::process::id()));
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

    pub fn bench(&self, path: &Path, iterations: u64) -> i32 {
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

        let benches = collect_benches(&m.program, m.entry_file, &mut m.diags);
        if m.diags.has_errors() {
            eprint!("{}", render::render_all(m.diags.diagnostics(), &m.sm));
            return 1;
        }
        if benches.is_empty() {
            eprintln!("tidak ada tolok ukur (nama 'tolok_*') di '{}'", path.display());
            return 1;
        }

        let span = m.program.header_span;
        for (sym, decl) in [("fflush", true), ("clock", false)] {
            let present = m.program.items.iter().any(
                |i| matches!(i, Item::Extern(e) if e.name.name == sym),
            );
            if !present {
                let d = if decl {
                    synth_fflush_decl(&mut m.next_node, span)
                } else {
                    synth_clock_decl(span)
                };
                m.program.items.push(Item::Extern(d));
            }
        }
        let main = synth_bench_main(&benches, iterations, &mut m.next_node, span);
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

        let work = std::env::temp_dir().join(format!("exotickik-bench-{}", std::process::id()));
        let out_exe = work.join(exe_stem(path, &self.target));
        let build_work = build_dir(&out_exe);
        if let Err(e) = build_executable(&module, &self.target, &out_exe, &build_work, self.opt_level) {
            eprintln!("error: kegagalan backend: {}", e);
            return 1;
        }

        let output = match std::process::Command::new(&out_exe).output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("error: tidak bisa menjalankan tolok ukur: {}", e);
                return 1;
            }
        };
        report_benches(path, &benches, iterations, &output)
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
        let pass = mk_print(next, span, &format!("##EXOTICKIK-UJI LULUS {}", name));
        let fail = mk_print(next, span, &format!("##EXOTICKIK-UJI GAGAL {}", name));
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
        if let Some(rest) = line.strip_prefix("##EXOTICKIK-UJI ") {
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

fn synth_clock_decl(span: Span) -> ExternDecl {
    ExternDecl {
        name: Ident { name: "clock".to_string(), span },
        params: Vec::new(),
        variadic: false,
        ret: Some(TypeExpr { kind: TypeExprKind::Named("angka".to_string()), span }),
        span,
    }
}

fn collect_benches(program: &Program, entry: FileId, diags: &mut DiagnosticEngine) -> Vec<String> {
    let mut names = Vec::new();
    for item in &program.items {
        let Item::Function(f) = item else { continue };
        if !f.name.name.starts_with("tolok_") {
            continue;
        }
        if f.span.file != entry {
            continue;
        }
        let returns_angka = matches!(
            &f.ret,
            Some(t) if matches!(&t.kind, TypeExprKind::Named(n) if n == "angka")
        );
        if !f.params.is_empty() {
            diags.emit(
                Diagnostic::error(codes::E_UNSUPPORTED, "tolok ukur tidak boleh punya parameter")
                    .primary(f.name.span, "hapus parameter dari fungsi tolok ukur ini"),
            );
        } else if !f.type_params.is_empty() {
            diags.emit(
                Diagnostic::error(codes::E_UNSUPPORTED, "tolok ukur tidak boleh generic")
                    .primary(f.name.span, "fungsi tolok ukur harus tanpa type parameter"),
            );
        } else if !returns_angka {
            diags.emit(
                Diagnostic::error(codes::E_UNSUPPORTED, "tolok ukur harus mengembalikan angka")
                    .primary(f.name.span, "kembalikan angka agar hasil kerja tidak dihapus optimizer"),
            );
        } else {
            names.push(f.name.name.clone());
        }
    }
    names
}

fn synth_bench_main(benches: &[String], iters: u64, next: &mut u32, span: Span) -> MainBlock {
    let mut stmts = Vec::new();
    for name in benches {
        let mut body = Vec::new();

        let clock0 = mk_call(next, span, "clock", Vec::new());
        body.push(mk_let(next, span, false, "t0", clock0));
        let zero = mk_expr(next, span, ExprKind::Int { value: 0, suffix: None });
        body.push(mk_let(next, span, true, "jumlah", zero));
        let zero_i = mk_expr(next, span, ExprKind::Int { value: 0, suffix: None });
        body.push(mk_let(next, span, true, "i", zero_i));

        let i_name = mk_name(next, span, "i");
        let limit = mk_expr(next, span, ExprKind::Int { value: iters as u128, suffix: None });
        let cond = mk_bin(next, span, BinOp::Lt, i_name, limit);

        let mut loop_body = Vec::new();
        let jumlah_name = mk_name(next, span, "jumlah");
        let call = mk_call(next, span, name, Vec::new());
        let add = mk_bin(next, span, BinOp::Add, jumlah_name, call);
        let jumlah_tgt = mk_name(next, span, "jumlah");
        loop_body.push(Stmt::Assign { target: jumlah_tgt, value: add, span });
        let i_rhs = mk_name(next, span, "i");
        let one = mk_expr(next, span, ExprKind::Int { value: 1, suffix: None });
        let inc = mk_bin(next, span, BinOp::Add, i_rhs, one);
        let i_tgt = mk_name(next, span, "i");
        loop_body.push(Stmt::Assign { target: i_tgt, value: inc, span });
        body.push(Stmt::While { cond, body: Block { stmts: loop_body, span }, span });

        let now = mk_call(next, span, "clock", Vec::new());
        let t0_name = mk_name(next, span, "t0");
        let dt = mk_bin(next, span, BinOp::Sub, now, t0_name);
        body.push(mk_let(next, span, false, "dt", dt));

        body.push(Stmt::Expr(mk_print(next, span, &format!("##EXOTICKIK-TOLOK {}", name))));
        let dt_name = mk_name(next, span, "dt");
        let dt_print = mk_call(next, span, "tampilkan", vec![dt_name]);
        body.push(Stmt::Expr(dt_print));
        let sum_name = mk_name(next, span, "jumlah");
        let sum_print = mk_call(next, span, "tampilkan", vec![sum_name]);
        body.push(Stmt::Expr(sum_print));
        let flush_zero = mk_expr(next, span, ExprKind::Int { value: 0, suffix: None });
        let flush = mk_call(next, span, "fflush", vec![flush_zero]);
        body.push(Stmt::Expr(flush));

        stmts.push(Stmt::Block(Block { stmts: body, span }));
    }
    MainBlock { body: Block { stmts, span }, span }
}

fn mk_name(next: &mut u32, span: Span, name: &str) -> Expr {
    mk_expr(next, span, ExprKind::Name(name.to_string()))
}

fn mk_bin(next: &mut u32, span: Span, op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    mk_expr(next, span, ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) })
}

fn mk_let(next: &mut u32, span: Span, mutable: bool, name: &str, init: Expr) -> Stmt {
    let id = NodeId(*next);
    *next += 1;
    Stmt::Let {
        mutable,
        name: Ident { name: name.to_string(), span },
        ty: None,
        init,
        span,
        id,
    }
}

fn report_benches(
    path: &Path,
    benches: &[String],
    iters: u64,
    output: &std::process::Output,
) -> i32 {
    use std::collections::HashMap;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut dt: HashMap<&str, i64> = HashMap::new();
    let mut sum: HashMap<&str, i64> = HashMap::new();
    let lines: Vec<&str> = stdout.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(name) = lines[i].strip_prefix("##EXOTICKIK-TOLOK ") {
            let d = lines.get(i + 1).and_then(|l| l.trim().parse::<i64>().ok());
            let s = lines.get(i + 2).and_then(|l| l.trim().parse::<i64>().ok());
            if let (Some(d), Some(s)) = (d, s) {
                dt.insert(name, d);
                sum.insert(name, s);
                i += 3;
                continue;
            }
        }
        i += 1;
    }

    println!("menjalankan {} tolok ukur: {} ({} iterasi)\n", benches.len(), path.display(), iters);
    let mut incomplete = false;
    for name in benches {
        match (dt.get(name.as_str()), sum.get(name.as_str())) {
            (Some(&d), Some(&s)) => {
                let total_ms = d as f64 / 1000.0;
                let ns_per_op = (d as f64 * 1000.0) / iters as f64;
                println!(
                    "  {:<24} {:>10.3} ms total   {:>12.3} ns/op   checksum={}",
                    name, total_ms, ns_per_op, s
                );
            }
            _ => {
                incomplete = true;
                println!("  {:<24} berhenti sebelum selesai", name);
            }
        }
    }
    if incomplete || !output.status.success() {
        1
    } else {
        0
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
            ModSource::Std(name) => format!("<pustaka>/{}.exo", name),
        }
    }
}

fn std_source(name: &str) -> Option<&'static str> {
    match name {
        "Inti" => Some(include_str!("../../../std/Inti.exo")),
        "Matematika" => Some(include_str!("../../../std/Matematika.exo")),
        "Teks" => Some(include_str!("../../../std/Teks.exo")),
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
        Some(p) => p.join(".exotickik-build"),
        None => PathBuf::from(".exotickik-build"),
    }
}
