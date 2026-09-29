use crate::results::*;
use lime_ast::*;
use lime_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use lime_span::Span;
use lime_types::{EnumId, StructId, Type};
use std::collections::HashMap;

pub struct Checker<'a> {
    diags: &'a mut DiagnosticEngine,
    results: TypeckResults,
    scopes: Vec<HashMap<String, Res>>,
    globals: HashMap<String, Res>,
    struct_ids: HashMap<String, StructId>,
    enum_ids: HashMap<String, EnumId>,
    generic_fns: Vec<Function>,
    instantiations: HashMap<(usize, Vec<Type>), usize>,
    mono_worklist: Vec<usize>,
    next_mono_node: u32,
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
            enum_ids: HashMap::new(),
            generic_fns: Vec::new(),
            instantiations: HashMap::new(),
            mono_worklist: Vec::new(),
            next_mono_node: 1_000_000_000,
            next_local: 0,
            current_ret: Type::Unit,
            loop_depth: 0,
        }
    }

    pub fn analyze(mut self, program: &Program) -> TypeckResults {
        self.collect_enums(program);
        self.collect_structs(program);
        self.resolve_struct_fields(program);
        self.collect_functions(program);
        self.register_globals();
        self.check_bodies(program);
        self.process_mono_worklist();
        self.check_main_presence(program);
        self.results
    }

    fn process_mono_worklist(&mut self) {
        while let Some(idx) = self.mono_worklist.pop() {
            let f = self.results.mono_functions[idx].clone();
            self.check_function(&f);
        }
    }

    fn collect_enums(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::Enum(e) = item {
                if self.enum_ids.contains_key(&e.name.name)
                    || self.struct_ids.contains_key(&e.name.name)
                {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_REDEFINITION,
                            format!("tipe '{}' didefinisikan lebih dari sekali", e.name.name),
                        )
                        .primary(e.name.span, "definisi ulang di sini"),
                    );
                    continue;
                }
                let mut seen: HashMap<String, ()> = HashMap::new();
                let mut variants = Vec::new();
                for v in &e.variants {
                    if seen.insert(v.name.clone(), ()).is_some() {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REDEFINITION,
                                format!("varian '{}' didefinisikan lebih dari sekali", v.name),
                            )
                            .primary(v.span, "varian ganda"),
                        );
                        continue;
                    }
                    variants.push(v.name.clone());
                }
                let id = EnumId(self.results.enums.len() as u32);
                self.enum_ids.insert(e.name.name.clone(), id);
                self.results.enums.push(EnumInfo {
                    name: e.name.name.clone(),
                    variants,
                    def_span: e.name.span,
                });
            }
        }
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
                Item::Function(f) if !f.type_params.is_empty() => {
                    if self.globals.contains_key(&f.name.name)
                        || self.generic_fns.iter().any(|g| g.name.name == f.name.name)
                    {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REDEFINITION,
                                format!("fungsi '{}' didefinisikan lebih dari sekali", f.name.name),
                            )
                            .primary(f.name.span, "definisi ulang di sini"),
                        );
                        continue;
                    }
                    self.generic_fns.push(f.clone());
                }
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
        for (gidx, f) in self.generic_fns.iter().enumerate() {
            self.globals.insert(f.name.name.clone(), Res::GenericFn(gidx));
        }
        for (name, id) in &self.struct_ids {
            self.globals.insert(name.clone(), Res::Struct(*id));
        }
        for (name, id) in &self.enum_ids {
            self.globals.insert(name.clone(), Res::Enum(*id));
        }
        self.globals.insert("tampilkan".to_string(), Res::Builtin(Builtin::Tampilkan));
        self.globals.insert("panjang".to_string(), Res::Builtin(Builtin::Panjang));
    }

    fn check_bodies(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function(f) if f.type_params.is_empty() => self.check_function(f),
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
            self.results.local_of_binding.insert(p.id, id);
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
            Stmt::Let { mutable, name, ty, init, id: let_id, .. } => {
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
                self.results.local_of_binding.insert(*let_id, id);
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
            Stmt::ForEach(fe) => self.check_foreach(fe),
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
        self.results.local_of_binding.insert(for_stmt.var_id, id);
        self.loop_depth += 1;
        self.check_block(&for_stmt.body);
        self.loop_depth -= 1;
        self.exit_scope();
    }

    fn check_foreach(&mut self, fe: &ForEachStmt) {
        let iter_ty = self.check_expr(&fe.iterable, None);
        let elem_ty = match &iter_ty {
            Type::Array { elem, .. } => (**elem).clone(),
            Type::Error => Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_ITERABLE,
                        format!("nilai bertipe '{}' tidak dapat diiterasi", other.display()),
                    )
                    .primary(fe.iterable.span, "hanya larik yang dapat diiterasi dengan 'pada'"),
                );
                Type::Error
            }
        };
        self.enter_scope();
        let id = self.alloc_local(fe.var.name.clone(), elem_ty, false);
        self.declare(&fe.var.name, Res::Local(id), fe.var.span);
        self.results.local_of_binding.insert(fe.var_id, id);
        self.loop_depth += 1;
        self.check_block(&fe.body);
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
                        Res::Enum(_) => Type::Error,
                        Res::GenericFn(_) => Type::Error,
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
            ExprKind::Field { base, field } => {
                if let ExprKind::Name(n) = &base.kind {
                    if let Some(&eid) = self.enum_ids.get(n) {
                        return self.check_enum_variant(expr.id, eid, field);
                    }
                }
                self.check_field(base, field)
            }
            ExprKind::StructLit { name, fields } => self.check_struct_lit(name, fields),
            ExprKind::ArrayLit(elems) => self.check_array_lit(elems, expected),
            ExprKind::Index { base, index } => self.check_index(base, index),
        }
    }

    fn check_array_lit(&mut self, elems: &[Expr], expected: Option<&Type>) -> Type {
        let expected_elem = match expected {
            Some(Type::Array { elem, .. }) => Some((**elem).clone()),
            _ => None,
        };
        if elems.is_empty() {
            match expected {
                Some(Type::Array { elem, .. }) => {
                    return Type::Array { elem: elem.clone(), size: 0 };
                }
                _ => {
                    return Type::Array { elem: Box::new(Type::Error), size: 0 };
                }
            }
        }
        let first = self.check_expr(&elems[0], expected_elem.as_ref());
        let elem_ty = if first.is_error() {
            expected_elem.clone().unwrap_or(Type::Error)
        } else {
            first
        };
        for e in &elems[1..] {
            let t = self.check_expr(e, Some(&elem_ty));
            if !t.assignable_to(&elem_ty) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TYPE_MISMATCH_ASSIGN,
                        format!(
                            "elemen larik bertipe '{}', diharapkan '{}'",
                            t.display(),
                            elem_ty.display()
                        ),
                    )
                    .primary(e.span, format!("bertipe '{}'", t.display())),
                );
            }
        }
        Type::Array { elem: Box::new(elem_ty), size: elems.len() as u64 }
    }

    fn check_index(&mut self, base: &Expr, index: &Expr) -> Type {
        let base_ty = self.check_expr(base, None);
        let idx_ty = self.check_expr(index, Some(&Type::I64));
        if !idx_ty.is_error() && !idx_ty.is_integer() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_INVALID_OPERAND,
                    format!("indeks larik harus bilangan bulat, bukan '{}'", idx_ty.display()),
                )
                .primary(index.span, "indeks tidak valid"),
            );
        }
        match base_ty {
            Type::Array { elem, .. } => (*elem).clone(),
            Type::Error => Type::Error,
            other => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_NOT_INDEXABLE,
                        format!("nilai bertipe '{}' tidak dapat diindeks", other.display()),
                    )
                    .primary(base.span, "hanya larik yang dapat diindeks"),
                );
                Type::Error
            }
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
                if !matches!(
                    lt,
                    Type::Int { .. }
                        | Type::Float(_)
                        | Type::Bool
                        | Type::Char
                        | Type::Str
                        | Type::Enum(_)
                ) {
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
            Some(Res::GenericFn(gidx)) => self.check_generic_call(gidx, call, callee, args),
            Some(Res::Builtin(Builtin::Panjang)) => {
                self.results.name_res.insert(callee.id, Res::Builtin(Builtin::Panjang));
                if args.len() != 1 {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_ARITY_MISMATCH,
                            format!("'panjang' butuh 1 argumen, diberi {}", args.len()),
                        )
                        .primary(call.span, "jumlah argumen salah"),
                    );
                    for a in args {
                        self.check_expr(a, None);
                    }
                    return Type::I64;
                }
                let at = self.check_expr(&args[0], None);
                if !at.is_error() && !matches!(at, Type::Array { .. } | Type::Str) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INVALID_OPERAND,
                            format!("'panjang' butuh larik atau teks, bukan '{}'", at.display()),
                        )
                        .primary(args[0].span, "bukan larik atau teks"),
                    );
                }
                Type::I64
            }
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
                        Type::Int { .. }
                            | Type::Float(_)
                            | Type::Bool
                            | Type::Char
                            | Type::Str
                            | Type::Enum(_)
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

    fn check_generic_call(
        &mut self,
        gidx: usize,
        call: &Expr,
        callee: &Expr,
        args: &[Expr],
    ) -> Type {
        let template = self.generic_fns[gidx].clone();
        let type_params: Vec<String> =
            template.type_params.iter().map(|p| p.name.clone()).collect();

        if args.len() != template.params.len() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_ARITY_MISMATCH,
                    format!(
                        "fungsi '{}' butuh {} argumen, diberi {}",
                        template.name.name,
                        template.params.len(),
                        args.len()
                    ),
                )
                .primary(call.span, "jumlah argumen salah"),
            );
            for a in args {
                self.check_expr(a, None);
            }
            return Type::Error;
        }

        let arg_types: Vec<Type> = args.iter().map(|a| self.check_expr(a, None)).collect();
        if arg_types.iter().any(|t| t.is_error()) {
            return Type::Error;
        }

        let mut subst: HashMap<String, Type> = HashMap::new();
        for (p, at) in template.params.iter().zip(arg_types.iter()) {
            let pat = self.resolve_type_generic(&p.ty, &type_params);
            if !unify(&pat, at, &mut subst) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_TYPE_MISMATCH_ASSIGN,
                        format!(
                            "argumen bertipe '{}' tidak cocok dengan parameter '{}'",
                            at.display(),
                            pat.display()
                        ),
                    )
                    .primary(p.ty.span, "tipe parameter")
                    .secondary(call.span, "pada pemanggilan ini"),
                );
                return Type::Error;
            }
        }

        let type_args: Vec<Type> = match type_params
            .iter()
            .map(|name| subst.get(name).cloned())
            .collect::<Option<Vec<_>>>()
        {
            Some(v) => v,
            None => {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_CANNOT_INFER,
                        format!(
                            "tidak dapat menyimpulkan parameter tipe untuk '{}'",
                            template.name.name
                        ),
                    )
                    .primary(call.span, "parameter tipe tidak tersimpulkan dari argumen"),
                );
                return Type::Error;
            }
        };

        let instance_idx = self.instantiate(gidx, &type_args);
        self.results.name_res.insert(callee.id, Res::Func(instance_idx));
        self.results.functions[instance_idx].ret.clone()
    }

    fn instantiate(&mut self, gidx: usize, type_args: &[Type]) -> usize {
        let key = (gidx, type_args.to_vec());
        if let Some(&idx) = self.instantiations.get(&key) {
            return idx;
        }
        let template = self.generic_fns[gidx].clone();
        let type_params: Vec<String> =
            template.type_params.iter().map(|p| p.name.clone()).collect();
        let subst: HashMap<String, Type> =
            type_params.iter().cloned().zip(type_args.iter().cloned()).collect();

        let mut inst = template.clone();
        inst.type_params.clear();
        inst.name.name = mangle_name(&template.name.name, type_args);
        substitute_fn_types(&mut inst, &subst, &self.struct_ids, &self.enum_ids, &self.results);
        self.renumber_fn(&mut inst);

        let params: Vec<(String, Type)> = inst
            .params
            .iter()
            .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
            .collect();
        let ret = inst.ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Type::Unit);
        let sig = FnSig {
            name: inst.name.name.clone(),
            params,
            ret,
            is_extern: false,
            variadic: false,
            public: false,
            def_span: template.name.span,
        };

        let instance_idx = self.results.functions.len();
        self.results.functions.push(sig);
        self.globals.insert(inst.name.name.clone(), Res::Func(instance_idx));

        let mono_idx = self.results.mono_functions.len();
        self.results.mono_functions.push(inst);
        self.mono_worklist.push(mono_idx);
        self.instantiations.insert(key, instance_idx);
        instance_idx
    }

    fn resolve_type_generic(&self, ty: &TypeExpr, type_params: &[String]) -> Type {
        match &ty.kind {
            TypeExprKind::Named(name) => {
                if type_params.iter().any(|p| p == name) {
                    return Type::Param(name.clone());
                }
                if let Some(t) = Type::from_builtin_name(name) {
                    return t;
                }
                if let Some(&id) = self.struct_ids.get(name) {
                    return Type::Struct(id);
                }
                if let Some(&id) = self.enum_ids.get(name) {
                    return Type::Enum(id);
                }
                Type::Error
            }
            TypeExprKind::Array { elem, size } => Type::Array {
                elem: Box::new(self.resolve_type_generic(elem, type_params)),
                size: *size,
            },
        }
    }

    fn renumber_fn(&mut self, f: &mut Function) {
        renumber_block(&mut f.body, &mut self.next_mono_node);
        for p in &mut f.params {
            p.id = NodeId(self.next_mono_node);
            self.next_mono_node += 1;
        }
    }

    fn check_enum_variant(&mut self, node: NodeId, eid: EnumId, variant: &Ident) -> Type {
        let info = self.results.enum_info(eid);
        match info.variant_tag(&variant.name) {
            Some(tag) => {
                self.results.enum_variant.insert(node, (eid, tag));
                Type::Enum(eid)
            }
            None => {
                let ename = info.name.clone();
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNKNOWN_VARIANT,
                        format!("enum '{}' tidak memiliki varian '{}'", ename, variant.name),
                    )
                    .primary(variant.span, "varian tidak dikenal"),
                );
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
            ExprKind::Index { base, .. } => self.check_assignable(base),
            _ => {}
        }
    }

    fn resolve_type(&mut self, ty: &TypeExpr) -> Type {
        match &ty.kind {
            TypeExprKind::Named(name) => {
                if let Some(t) = Type::from_builtin_name(name) {
                    return t;
                }
                if let Some(&id) = self.struct_ids.get(name) {
                    return Type::Struct(id);
                }
                if let Some(&id) = self.enum_ids.get(name) {
                    return Type::Enum(id);
                }
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_UNKNOWN_TYPE,
                        format!("tipe '{}' tidak dikenal", name),
                    )
                    .primary(ty.span, "tipe tidak ditemukan"),
                );
                Type::Error
            }
            TypeExprKind::Array { elem, size } => {
                let elem_ty = self.resolve_type(elem);
                Type::Array { elem: Box::new(elem_ty), size: *size }
            }
        }
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

fn unify(pat: &Type, actual: &Type, subst: &mut HashMap<String, Type>) -> bool {
    match pat {
        Type::Param(name) => match subst.get(name) {
            Some(bound) => bound == actual,
            None => {
                subst.insert(name.clone(), actual.clone());
                true
            }
        },
        Type::Array { elem, size } => {
            if let Type::Array { elem: e2, size: s2 } = actual {
                size == s2 && unify(elem, e2, subst)
            } else {
                false
            }
        }
        other => other == actual,
    }
}

fn mangle_name(base: &str, type_args: &[Type]) -> String {
    let parts: Vec<String> = type_args
        .iter()
        .map(|t| t.display().chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect())
        .collect();
    format!("{}${}", base, parts.join("_"))
}

fn type_to_type_expr_kind(ty: &Type, results: &TypeckResults) -> TypeExprKind {
    match ty {
        Type::Struct(id) => TypeExprKind::Named(results.structs[id.0 as usize].name.clone()),
        Type::Enum(id) => TypeExprKind::Named(results.enums[id.0 as usize].name.clone()),
        Type::Array { elem, size } => TypeExprKind::Array {
            elem: Box::new(TypeExpr {
                kind: type_to_type_expr_kind(elem, results),
                span: Span::point(lime_span::FileId(0), 0),
            }),
            size: *size,
        },
        other => TypeExprKind::Named(other.display()),
    }
}

fn substitute_type_expr(te: &mut TypeExpr, subst: &HashMap<String, Type>, results: &TypeckResults) {
    let replacement = if let TypeExprKind::Named(name) = &te.kind {
        subst.get(name).map(|c| type_to_type_expr_kind(c, results))
    } else {
        None
    };
    if let Some(kind) = replacement {
        te.kind = kind;
        return;
    }
    if let TypeExprKind::Array { elem, .. } = &mut te.kind {
        substitute_type_expr(elem, subst, results);
    }
}

fn substitute_fn_types(
    f: &mut Function,
    subst: &HashMap<String, Type>,
    _struct_ids: &HashMap<String, StructId>,
    _enum_ids: &HashMap<String, EnumId>,
    results: &TypeckResults,
) {
    for p in &mut f.params {
        substitute_type_expr(&mut p.ty, subst, results);
    }
    if let Some(ret) = &mut f.ret {
        substitute_type_expr(ret, subst, results);
    }
    substitute_block_lets(&mut f.body, subst, results);
}

fn substitute_block_lets(
    block: &mut Block,
    subst: &HashMap<String, Type>,
    results: &TypeckResults,
) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Let { ty: Some(t), .. } => substitute_type_expr(t, subst, results),
            Stmt::If(i) => {
                substitute_block_lets(&mut i.then_block, subst, results);
                for (_, b) in &mut i.else_ifs {
                    substitute_block_lets(b, subst, results);
                }
                if let Some(b) = &mut i.else_block {
                    substitute_block_lets(b, subst, results);
                }
            }
            Stmt::While { body, .. } => substitute_block_lets(body, subst, results),
            Stmt::For(f) => substitute_block_lets(&mut f.body, subst, results),
            Stmt::ForEach(f) => substitute_block_lets(&mut f.body, subst, results),
            Stmt::Block(b) => substitute_block_lets(b, subst, results),
            _ => {}
        }
    }
}

fn renumber_block(block: &mut Block, counter: &mut u32) {
    for stmt in &mut block.stmts {
        renumber_stmt(stmt, counter);
    }
}

fn renumber_stmt(stmt: &mut Stmt, counter: &mut u32) {
    match stmt {
        Stmt::Let { init, id, .. } => {
            renumber_expr(init, counter);
            *id = NodeId(*counter);
            *counter += 1;
        }
        Stmt::Assign { target, value, .. } => {
            renumber_expr(target, counter);
            renumber_expr(value, counter);
        }
        Stmt::Expr(e) => renumber_expr(e, counter),
        Stmt::If(i) => {
            renumber_expr(&mut i.cond, counter);
            renumber_block(&mut i.then_block, counter);
            for (c, b) in &mut i.else_ifs {
                renumber_expr(c, counter);
                renumber_block(b, counter);
            }
            if let Some(b) = &mut i.else_block {
                renumber_block(b, counter);
            }
        }
        Stmt::While { cond, body, .. } => {
            renumber_expr(cond, counter);
            renumber_block(body, counter);
        }
        Stmt::For(f) => {
            renumber_expr(&mut f.start, counter);
            renumber_expr(&mut f.end, counter);
            f.var_id = NodeId(*counter);
            *counter += 1;
            renumber_block(&mut f.body, counter);
        }
        Stmt::ForEach(f) => {
            renumber_expr(&mut f.iterable, counter);
            f.var_id = NodeId(*counter);
            *counter += 1;
            renumber_block(&mut f.body, counter);
        }
        Stmt::Return { value: Some(e), .. } => renumber_expr(e, counter),
        Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Block(b) => renumber_block(b, counter),
    }
}

fn renumber_expr(e: &mut Expr, counter: &mut u32) {
    e.id = NodeId(*counter);
    *counter += 1;
    match &mut e.kind {
        ExprKind::Binary { lhs, rhs, .. } => {
            renumber_expr(lhs, counter);
            renumber_expr(rhs, counter);
        }
        ExprKind::Unary { operand, .. } => renumber_expr(operand, counter),
        ExprKind::Call { callee, args } => {
            renumber_expr(callee, counter);
            for a in args {
                renumber_expr(a, counter);
            }
        }
        ExprKind::Field { base, .. } => renumber_expr(base, counter),
        ExprKind::StructLit { fields, .. } => {
            for f in fields {
                renumber_expr(&mut f.value, counter);
            }
        }
        ExprKind::ArrayLit(elems) => {
            for a in elems {
                renumber_expr(a, counter);
            }
        }
        ExprKind::Index { base, index } => {
            renumber_expr(base, counter);
            renumber_expr(index, counter);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Str(_)
        | ExprKind::Char(_)
        | ExprKind::Bool(_)
        | ExprKind::Name(_) => {}
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
        Stmt::ForEach(f) => f.span,
        Stmt::Return { span, .. } => *span,
        Stmt::Break(s) => *s,
        Stmt::Continue(s) => *s,
        Stmt::Block(b) => b.span,
    }
}
