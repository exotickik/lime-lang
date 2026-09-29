use crate::results::*;
use lime_ast::*;
use lime_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use lime_span::Span;
use lime_types::{StructId, Type};
use std::collections::HashMap;

pub struct Checker<'a> {
    diags: &'a mut DiagnosticEngine,
    results: TypeckResults,
    scopes: Vec<HashMap<String, Res>>,
    globals: HashMap<String, Res>,
    struct_ids: HashMap<String, StructId>,
    next_local: u32,
    current_ret: Type,
    loop_depth: u32,
}

impl<'a> Checker<'a> {
    pub fn new(diags: &'a mut DiagnosticEngine) -> Checker<'a> {
        Checker {
            diags,
            results: TypeckResults::default(),
            scopes: Vec::new(),
            globals: HashMap::new(),
            struct_ids: HashMap::new(),
            next_local: 0,
            current_ret: Type::Unit,
            loop_depth: 0,
        }
    }

    pub fn analyze(mut self, program: &Program) -> TypeckResults {
        self.collect_structs(program);
        self.resolve_struct_fields(program);
        self.collect_functions(program);
        self.register_globals();
        self.check_bodies(program);
        self.check_main_presence(program);
        self.results
    }

    fn collect_structs(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Struct(s) = item {
                if self.struct_ids.contains_key(&s.name.name) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_REDEFINITION,
                            format!("tipe '{}' didefinisikan lebih dari sekali", s.name.name),
                        )
                        .primary(s.name.span, "definisi ulang di sini"),
                    );
                    continue;
                }
                let id = StructId(self.results.structs.len() as u32);
                self.struct_ids.insert(s.name.name.clone(), id);
                self.results.structs.push(StructInfo {
                    name: s.name.name.clone(),
                    fields: Vec::new(),
                    def_span: s.name.span,
                });
            }
        }
    }

    fn resolve_struct_fields(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Struct(s) = item {
                let Some(&id) = self.struct_ids.get(&s.name.name) else { continue };
                let mut fields = Vec::new();
                let mut seen = HashMap::new();
                for f in &s.fields {
                    if seen.insert(f.name.name.clone(), ()).is_some() {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REDEFINITION,
                                format!("field '{}' didefinisikan lebih dari sekali", f.name.name),
                            )
                            .primary(f.name.span, "field ganda"),
                        );
                    }
                    let ty = self.resolve_type(&f.ty);
                    fields.push(FieldInfo { name: f.name.name.clone(), ty, public: f.public });
                }
                self.results.structs[id.0 as usize].fields = fields;
            }
        }
    }

    fn collect_functions(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) => {
                    let params = f
                        .params
                        .iter()
                        .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
                        .collect();
                    let ret = f.ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Type::Unit);
                    self.push_fn(FnSig {
                        name: f.name.name.clone(),
                        params,
                        ret,
                        is_extern: false,
                        variadic: false,
                        public: f.public,
                        def_span: f.name.span,
                    });
                }
                Item::Extern(e) => {
                    let params = e
                        .params
                        .iter()
                        .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
                        .collect();
                    let ret = e.ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Type::Unit);
                    self.push_fn(FnSig {
                        name: e.name.name.clone(),
                        params,
                        ret,
                        is_extern: true,
                        variadic: e.variadic,
                        public: true,
                        def_span: e.name.span,
                    });
                }
                _ => {}
            }
        }
    }

    fn push_fn(&mut self, sig: FnSig) {
        if self.results.func_by_name(&sig.name).is_some() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_REDEFINITION,
                    format!("fungsi '{}' didefinisikan lebih dari sekali", sig.name),
                )
                .primary(sig.def_span, "definisi ulang di sini"),
            );
            return;
        }
        self.results.functions.push(sig);
    }

    fn register_globals(&mut self) {
        for (idx, f) in self.results.functions.iter().enumerate() {
            self.globals.insert(f.name.clone(), Res::Func(idx));
        }
        for (name, id) in &self.struct_ids {
            self.globals.insert(name.clone(), Res::Struct(*id));
        }
        self.globals.insert("tampilkan".to_string(), Res::Builtin(Builtin::Tampilkan));
    }

    fn check_bodies(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) => self.check_function(f),
                Item::Main(m) => self.check_main(m),
                _ => {}
            }
        }
    }

    fn check_function(&mut self, f: &Function) {
        let sig = self.results.func_by_name(&f.name.name).map(|(_, s)| s.clone());
        let Some(sig) = sig else { return };
        self.current_ret = sig.ret.clone();
        self.loop_depth = 0;
        self.enter_scope();
        for (p, (pname, pty)) in f.params.iter().zip(sig.params.iter()) {
            let id = self.alloc_local(pname.clone(), pty.clone(), false);
            self.declare(&p.name.name, Res::Local(id), p.name.span);
            self.results.local_of_binding.insert(p.name.span, id);
        }
        self.check_block(&f.body);
        self.exit_scope();

        if sig.ret != Type::Unit && !block_always_returns(&f.body) {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_MISSING_RETURN,
                    format!(
                        "fungsi '{}' tidak mengembalikan '{}' pada semua jalur",
                        f.name.name,
                        sig.ret.display()
                    ),
                )
                .primary(f.name.span, "sebagian jalur tidak memiliki 'kembalikan'")
                .help("tambahkan 'kembalikan <nilai>' di akhir setiap cabang"),
            );
        }
    }

    fn check_main(&mut self, m: &MainBlock) {
        self.current_ret = Type::I64;
        self.loop_depth = 0;
        self.enter_scope();
        self.check_block(&m.body);
        self.exit_scope();
    }

    fn check_main_presence(&mut self, program: &Program) {
        let mains: Vec<&MainBlock> = program
            .items
            .iter()
            .filter_map(|i| if let Item::Main(m) = i { Some(m) } else { None })
            .collect();
        self.results.main_present = !mains.is_empty();
        if mains.is_empty() {
            self.diags.emit(
                Diagnostic::error(codes::E_MISSING_MAIN, "program tidak memiliki blok 'utama'")
                    .primary(program.header_span, "tambahkan blok 'utama { ... }'"),
            );
        } else if mains.len() > 1 {
            for extra in &mains[1..] {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_DUPLICATE_MAIN,
                        "hanya boleh ada satu blok 'utama'",
                    )
                    .primary(extra.span, "blok 'utama' tambahan"),
                );
            }
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.enter_scope();
        let mut terminated: Option<Span> = None;
        for stmt in &block.stmts {
            if let Some(_prev) = terminated {
                self.diags.emit(
                    Diagnostic::warning(codes::W_UNREACHABLE, "kode ini tidak akan dieksekusi")
                        .primary(stmt_span(stmt), "tidak terjangkau"),
                );
                terminated = None;
            }
            self.check_stmt(stmt);
            if stmt_is_terminator(stmt) {
                terminated = Some(stmt_span(stmt));
            }
        }
        self.exit_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { mutable, name, ty, init, .. } => {
                let annotated = ty.as_ref().map(|t| self.resolve_type(t));
                let init_ty = self.check_expr(init, annotated.as_ref());
                let decl_ty = annotated.clone().unwrap_or_else(|| init_ty.clone());
                if let Some(ann) = &annotated {
                    if !init_ty.assignable_to(ann) {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_TYPE_MISMATCH_ASSIGN,
                                format!(
                                    "tipe nilai awal '{}' tidak cocok dengan '{}'",
                                    init_ty.display(),
                                    ann.display()
                                ),
                            )
                            .primary(init.span, format!("bertipe '{}'", init_ty.display()))
                            .secondary(name.span, format!("dideklarasikan '{}'", ann.display())),
                        );
                    }
                }
                let id = self.alloc_local(name.name.clone(), decl_ty, *mutable);
                self.declare(&name.name, Res::Local(id), name.span);
                self.results.local_of_binding.insert(name.span, id);
            }
            Stmt::Assign { target, value, .. } => {
                let target_ty = self.check_expr(target, None);
                self.check_assignable(target);
                let value_ty = self.check_expr(value, Some(&target_ty));
                if !value_ty.assignable_to(&target_ty) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_TYPE_MISMATCH_ASSIGN,
                            format!(
                                "tidak dapat menetapkan '{}' ke target '{}'",
                                value_ty.display(),
                                target_ty.display()
                            ),
                        )
                        .primary(value.span, format!("bertipe '{}'", value_ty.display())),
                    );
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(e, None);
            }
            Stmt::If(if_stmt) => self.check_if(if_stmt),
            Stmt::While { cond, body, .. } => {
                let ct = self.check_expr(cond, Some(&Type::Bool));
                self.expect_bool(&ct, cond.span, "kondisi 'selama'");
                self.loop_depth += 1;
                self.check_block(body);
                self.loop_depth -= 1;
            }
            Stmt::For(for_stmt) => self.check_for(for_stmt),
            Stmt::Return { value, span } => {
                let ret = self.current_ret.clone();
                match value {
                    Some(e) => {
                        let t = self.check_expr(e, Some(&ret));
                        if !t.assignable_to(&ret) {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_RETURN_TYPE_MISMATCH,
                                    format!(
                                        "mengembalikan '{}' tetapi fungsi bertipe '{}'",
                                        t.display(),
                                        ret.display()
                                    ),
                                )
                                .primary(e.span, format!("bertipe '{}'", t.display())),
                            );
                        }
                    }
                    None => {
                        if ret != Type::Unit && ret != Type::I64 {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_RETURN_TYPE_MISMATCH,
                                    format!(
                                        "'kembalikan' tanpa nilai tetapi fungsi bertipe '{}'",
                                        ret.display()
                                    ),
                                )
                                .primary(*span, "nilai kembalian hilang"),
                            );
                        }
                    }
                }
            }
            Stmt::Break(sp) => {
                if self.loop_depth == 0 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_LOOP_CTRL_OUTSIDE_LOOP,
                            "'berhenti' hanya boleh di dalam perulangan",
                        )
                        .primary(*sp, "di luar perulangan"),
                    );
                }
            }
            Stmt::Continue(sp) => {
                if self.loop_depth == 0 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_LOOP_CTRL_OUTSIDE_LOOP,
                            "'lanjut' hanya boleh di dalam perulangan",
                        )
                        .primary(*sp, "di luar perulangan"),
                    );
                }
            }
            Stmt::Block(b) => self.check_block(b),
        }
    }

    fn check_if(&mut self, if_stmt: &IfStmt) {
        let ct = self.check_expr(&if_stmt.cond, Some(&Type::Bool));
        self.expect_bool(&ct, if_stmt.cond.span, "kondisi 'jika'");
        self.check_block(&if_stmt.then_block);
        for (cond, block) in &if_stmt.else_ifs {
            let t = self.check_expr(cond, Some(&Type::Bool));
            self.expect_bool(&t, cond.span, "kondisi 'lain jika'");
            self.check_block(block);
        }
        if let Some(block) = &if_stmt.else_block {
            self.check_block(block);
        }
    }

    fn check_for(&mut self, for_stmt: &ForStmt) {
        let start_ty = self.check_expr(&for_stmt.start, Some(&Type::I64));
        let var_ty = if start_ty.is_integer() {
            start_ty.clone()
        } else {
            if !start_ty.is_error() {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_INVALID_OPERAND,
                        "batas awal perulangan harus bilangan bulat",
                    )
                    .primary(for_stmt.start.span, format!("bertipe '{}'", start_ty.display())),
                );
            }
            Type::I64
        };
        let end_ty = self.check_expr(&for_stmt.end, Some(&var_ty));
        if !end_ty.assignable_to(&var_ty) && !end_ty.is_error() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_TYPE_MISMATCH_BINOP,
                    format!(
                        "batas rentang bertipe berbeda: '{}' dan '{}'",
                        var_ty.display(),
                        end_ty.display()
                    ),
                )
                .primary(for_stmt.end.span, format!("bertipe '{}'", end_ty.display())),
            );
        }
        self.enter_scope();
        let id = self.alloc_local(for_stmt.var.name.clone(), var_ty, false);
        self.declare(&for_stmt.var.name, Res::Local(id), for_stmt.var.span);
        self.results.local_of_binding.insert(for_stmt.var.span, id);
        self.loop_depth += 1;
        self.check_block(&for_stmt.body);
        self.loop_depth -= 1;
        self.exit_scope();
    }

    fn check_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        let ty = self.check_expr_inner(expr, expected);
        self.results.expr_types.insert(expr.id, ty.clone());
        ty
    }

    fn check_expr_inner(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        match &expr.kind {
            ExprKind::Int { suffix, .. } => {
                if let Some(s) = suffix {
                    Type::from_builtin_name(s).unwrap_or(Type::I64)
                } else if let Some(exp) = expected {
                    if exp.is_numeric() {
                        exp.clone()
                    } else {
                        Type::I64
                    }
                } else {
                    Type::I64
                }
            }
            ExprKind::Float { suffix, .. } => {
                if let Some(s) = suffix {
                    Type::from_builtin_name(s).unwrap_or(Type::F64)
                } else if let Some(exp) = expected {
                    if exp.is_float() {
                        exp.clone()
                    } else {
                        Type::F64
                    }
                } else {
                    Type::F64
                }
            }
            ExprKind::Str(_) => Type::Str,
            ExprKind::Char(_) => Type::Char,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Name(name) => match self.lookup(name) {
                Some(res) => {
                    self.results.name_res.insert(expr.id, res.clone());
                    match res {
                        Res::Local(id) => self.results.locals[&id].ty.clone(),
                        Res::Func(idx) => self.results.functions[idx].fn_type(),
                        Res::Builtin(_) => Type::Error,
                        Res::Struct(_) => Type::Error,
                    }
                }
                None => {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_UNDEFINED_NAME,
                            format!("nama '{}' belum didefinisikan", name),
                        )
                        .primary(expr.span, "tidak ditemukan pada scope ini"),
                    );
                    Type::Error
                }
            },
            ExprKind::Binary { op, lhs, rhs } => self.check_binary(*op, lhs, rhs, expected),
            ExprKind::Unary { op, operand } => self.check_unary(*op, operand),
            ExprKind::Call { callee, args } => self.check_call(expr, callee, args),
            ExprKind::Field { base, field } => self.check_field(base, field),
            ExprKind::StructLit { name, fields } => self.check_struct_lit(name, fields),
        }
    }

    fn check_struct_lit(&mut self, name: &Ident, fields: &[StructLitField]) -> Type {
        let id = match self.struct_ids.get(&name.name) {
            Some(&id) => id,
            None => {
                let msg = if Type::from_builtin_name(&name.name).is_some() {
                    format!("tipe bawaan '{}' tidak dapat dibangun dengan '{{ }}'", name.name)
                } else {
                    format!("'{}' bukan tipe struct", name.name)
                };
                self.diags.emit(
                    Diagnostic::error(codes::E_NOT_A_STRUCT, msg)
                        .primary(name.span, "bukan struct yang dikenal"),
                );
                for f in fields {
                    self.check_expr(&f.value, None);
                }
                return Type::Error;
            }
        };

        let info = self.results.struct_info(id).clone();
        let mut seen: HashMap<String, Span> = HashMap::new();

        for f in fields {
            if let Some(prev) = seen.insert(f.name.name.clone(), f.name.span) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_DUPLICATE_FIELD,
                        format!("field '{}' diisi lebih dari sekali", f.name.name),
                    )
                    .primary(f.name.span, "pengisian ganda")
                    .secondary(prev, "pertama kali di sini"),
                );
            }
            match info.field(&f.name.name) {
                Some((_, field_info)) => {
                    let vty = self.check_expr(&f.value, Some(&field_info.ty));
                    if !vty.assignable_to(&field_info.ty) {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_TYPE_MISMATCH_ASSIGN,
                                format!(
                                    "field '{}' bertipe '{}', diberi '{}'",
                                    f.name.name,
                                    field_info.ty.display(),
                                    vty.display()
                                ),
                            )
                            .primary(f.value.span, format!("bertipe '{}'", vty.display())),
                        );
                    }
                }
                None => {
                    self.check_expr(&f.value, None);
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_UNKNOWN_FIELD,
                            format!("tipe '{}' tidak memiliki field '{}'", info.name, f.name.name),
                        )
                        .primary(f.name.span, "field tidak dikenal"),
                    );
                }
            }
        }

        let missing: Vec<String> = info
            .fields
            .iter()
            .filter(|fi| !seen.contains_key(&fi.name))
            .map(|fi| fi.name.clone())
            .collect();
        if !missing.is_empty() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_MISSING_FIELD,
                    format!("field belum diisi: {}", missing.join(", ")),
                )
                .primary(name.span, format!("struct '{}' butuh semua field", info.name)),
            );
        }

        Type::Struct(id)
    }

    fn check_binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, expected: Option<&Type>) -> Type {
        if op.is_logical() {
            let lt = self.check_expr(lhs, Some(&Type::Bool));
            let rt = self.check_expr(rhs, Some(&Type::Bool));
            self.expect_bool(&lt, lhs.span, "operand logika");
            self.expect_bool(&rt, rhs.span, "operand logika");
            return Type::Bool;
        }

        let lt = self.check_expr(lhs, if op.is_arithmetic() { expected } else { None });
        let rhs_expected = if lt.is_error() { None } else { Some(lt.clone()) };
        let rt = self.check_expr(rhs, rhs_expected.as_ref());

        if lt.is_error() || rt.is_error() {
            return if op.is_comparison() { Type::Bool } else { Type::Error };
        }

        if lt != rt {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_TYPE_MISMATCH_BINOP,
                    format!(
                        "operand '{}' tidak cocok: '{}' dan '{}'",
                        op.symbol(),
                        lt.display(),
                        rt.display()
                    ),
                )
                .primary(lhs.span.to(rhs.span), "tipe kedua sisi harus sama")
                .help("Lime tidak melakukan konversi numerik implisit"),
            );
            return if op.is_comparison() { Type::Bool } else { Type::Error };
        }

        if op.is_comparison() {
            if op == BinOp::Eq || op == BinOp::Ne {
                if !matches!(lt, Type::Int { .. } | Type::Float(_) | Type::Bool | Type::Char | Type::Str)
                {
                    self.invalid_operand(op, lhs.span.to(rhs.span), &lt);
                    return Type::Bool;
                }
            } else if !lt.is_numeric() {
                self.invalid_operand(op, lhs.span.to(rhs.span), &lt);
            }
            return Type::Bool;
        }

        if !lt.is_numeric() {
            self.invalid_operand(op, lhs.span.to(rhs.span), &lt);
            return Type::Error;
        }
        lt
    }

    fn check_unary(&mut self, op: UnOp, operand: &Expr) -> Type {
        match op {
            UnOp::Neg => {
                let t = self.check_expr(operand, None);
                if t.is_error() {
                    return Type::Error;
                }
                if !t.is_numeric() {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_OPERAND,
                            format!("negasi '-' butuh tipe numerik, bukan '{}'", t.display()),
                        )
                        .primary(operand.span, "operand tidak numerik"),
                    );
                    return Type::Error;
                }
                t
            }
            UnOp::Not => {
                let t = self.check_expr(operand, Some(&Type::Bool));
                self.expect_bool(&t, operand.span, "operand 'bukan'");
                Type::Bool
            }
        }
    }

    fn check_call(&mut self, call: &Expr, callee: &Expr, args: &[Expr]) -> Type {
        let ExprKind::Name(name) = &callee.kind else {
            self.check_expr(callee, None);
            self.diags.emit(
                Diagnostic::error(codes::E_NOT_CALLABLE, "ekspresi ini tidak dapat dipanggil")
                    .primary(callee.span, "bukan sebuah fungsi"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        };

        match self.lookup(name) {
            Some(Res::Builtin(Builtin::Tampilkan)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Tampilkan));
                if args.len() != 1 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!("'tampilkan' butuh 1 argumen, diberi {}", args.len()),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Type::Unit;
                }
                let at = self.check_expr(&args[0], None);
                if !at.is_error()
                    && !matches!(
                        at,
                        Type::Int { .. } | Type::Float(_) | Type::Bool | Type::Char | Type::Str
                    )
                {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_OPERAND,
                            format!("'tampilkan' tidak mendukung tipe '{}'", at.display()),
                        )
                        .primary(args[0].span, "tipe tidak dapat dicetak"),
                    );
                }
                Type::Unit
            }
            Some(Res::Func(idx)) => {
                self.results.name_res.insert(callee.id, Res::Func(idx));
                let sig = self.results.functions[idx].clone();
                let ok_arity = if sig.variadic {
                    args.len() >= sig.params.len()
                } else {
                    args.len() == sig.params.len()
                };
                if !ok_arity {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!(
                                "fungsi '{}' butuh {} argumen, diberi {}",
                                sig.name,
                                sig.params.len(),
                                args.len()
                            ),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                }
                for (i, a) in args.iter().enumerate() {
                    let expected = sig.params.get(i).map(|(_, t)| t.clone());
                    let at = self.check_expr(a, expected.as_ref());
                    if let Some(pt) = expected {
                        if !at.assignable_to(&pt) {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_TYPE_MISMATCH_ASSIGN,
                                    format!(
                                        "argumen ke-{} bertipe '{}', diharapkan '{}'",
                                        i + 1,
                                        at.display(),
                                        pt.display()
                                    ),
                                )
                                .primary(a.span, format!("bertipe '{}'", at.display())),
                            );
                        }
                    }
                }
                sig.ret.clone()
            }
            Some(_) | None => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_CALLABLE,
                        format!("'{}' bukan fungsi yang dapat dipanggil", name),
                    )
                    .primary(callee.span, "bukan fungsi"),
                );
                for a in args {
                    self.check_expr(a, None);
                }
                Type::Error
            }
        }
    }

    fn check_field(&mut self, base: &Expr, field: &Ident) -> Type {
        let bt = self.check_expr(base, None);
        match bt {
            Type::Struct(id) => {
                let info = self.results.struct_info(id);
                match info.field(&field.name) {
                    Some((_, f)) => f.ty.clone(),
                    None => {
                        let sname = info.name.clone();
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_UNKNOWN_FIELD,
                                format!("tipe '{}' tidak memiliki field '{}'", sname, field.name),
                            )
                            .primary(field.span, "field tidak dikenal"),
                        );
                        Type::Error
                    }
                }
            }
            Type::Error => Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_A_STRUCT,
                        format!("nilai bertipe '{}' bukan struct", other.display()),
                    )
                    .primary(base.span, "akses field hanya untuk struct"),
                );
                Type::Error
            }
        }
    }

    fn check_assignable(&mut self, target: &Expr) {
        match &target.kind {
            ExprKind::Name(_) => match self.results.name_res.get(&target.id) {
                Some(Res::Local(id)) => {
                    let info = &self.results.locals[id];
                    if !info.mutable {
                        let nm = info.name.clone();
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_ASSIGN_IMMUTABLE,
                                format!("tidak dapat menetapkan ke '{}' yang immutable", nm),
                            )
                            .primary(target.span, "dideklarasikan dengan 'nilai'")
                            .help("gunakan 'ubah' agar dapat diubah"),
                        );
                    }
                }
                Some(_) => {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_ASSIGN_TARGET,
                            "target penugasan tidak valid",
                        )
                        .primary(target.span, "bukan variabel"),
                    );
                }
                None => {}
            },
            ExprKind::Field { base, .. } => self.check_assignable(base),
            _ => {}
        }
    }

    fn resolve_type(&mut self, ty: &TypeExpr) -> Type {
        if let Some(t) = Type::from_builtin_name(&ty.name) {
            return t;
        }
        if let Some(&id) = self.struct_ids.get(&ty.name) {
            return Type::Struct(id);
        }
        self.diags.emit(
            Diagnostic::error(codes::E_UNKNOWN_TYPE, format!("tipe '{}' tidak dikenal", ty.name))
                .primary(ty.span, "tipe tidak ditemukan"),
        );
        Type::Error
    }

    fn expect_bool(&mut self, t: &Type, span: Span, ctx: &str) {
        if !t.is_error() && *t != Type::Bool {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_INVALID_OPERAND,
                    format!("{} harus 'boolean', bukan '{}'", ctx, t.display()),
                )
                .primary(span, format!("bertipe '{}'", t.display())),
            );
        }
    }

    fn invalid_operand(&mut self, op: BinOp, span: Span, t: &Type) {
        self.diags.emit(
            Diagnostic::error(
                codes::E_INVALID_OPERAND,
                format!("operator '{}' tidak mendukung tipe '{}'", op.symbol(), t.display()),
            )
            .primary(span, "tipe operand tidak valid"),
        );
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, res: Res, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_REDEFINITION,
                        format!("'{}' sudah dideklarasikan pada scope ini", name),
                    )
                    .primary(span, "deklarasi ganda"),
                );
                return;
            }
            scope.insert(name.to_string(), res);
        }
    }

    fn lookup(&self, name: &str) -> Option<Res> {
        for scope in self.scopes.iter().rev() {
            if let Some(res) = scope.get(name) {
                return Some(res.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    fn alloc_local(&mut self, name: String, ty: Type, mutable: bool) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        self.results.locals.insert(id, LocalInfo { name, ty, mutable });
        id
    }
}

fn block_always_returns(block: &Block) -> bool {
    block.stmts.iter().any(stmt_diverges)
}

fn stmt_diverges(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::Block(b) => block_always_returns(b),
        Stmt::If(i) => if_diverges(i),
        _ => false,
    }
}

fn if_diverges(i: &IfStmt) -> bool {
    i.else_block.is_some()
        && block_always_returns(&i.then_block)
        && i.else_ifs.iter().all(|(_, b)| block_always_returns(b))
        && i.else_block.as_ref().map(block_always_returns).unwrap_or(false)
}

fn stmt_is_terminator(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_) => true,
        Stmt::If(i) => if_diverges(i),
        _ => false,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Let { span, .. } => *span,
        Stmt::Assign { span, .. } => *span,
        Stmt::Expr(e) => e.span,
        Stmt::If(i) => i.span,
        Stmt::While { span, .. } => *span,
        Stmt::For(f) => f.span,
        Stmt::Return { span, .. } => *span,
        Stmt::Break(s) => *s,
        Stmt::Continue(s) => *s,
        Stmt::Block(b) => b.span,
    }
}
