use crate::results::{Builtin, LocalId, RecvMode, Res, TypeckResults};
use lime_ast::*;
use lime_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use lime_span::Span;
use lime_types::Type;
use std::collections::{HashMap, HashSet};

pub fn is_copy(ty: &Type, results: &TypeckResults) -> bool {
    match ty {
        Type::Kotak(_) => false,
        Type::Struct(id) => {
            results.structs[id.0 as usize].fields.iter().all(|f| is_copy(&f.ty, results))
        }
        Type::Array { elem, .. } => is_copy(elem, results),
        Type::Ref { .. } => true,
        _ => true,
    }
}

pub fn check_program(program: &Program, results: &TypeckResults, diags: &mut DiagnosticEngine) -> HashSet<NodeId> {
    let mut checker = Own {
        results,
        diags,
        moved: HashMap::new(),
        declared: HashSet::new(),
        moved_uses: HashSet::new(),
        stmt_loans: Vec::new(),
        block_loans: Vec::new(),
        borrow_is_binding: false,
    };
    for item in &program.items {
        match item {
            Item::Function(f) if f.type_params.is_empty() => checker.check_fn(f),
            Item::Main(m) => {
                checker.reset_fn();
                checker.check_block(&m.body);
            }
            Item::Extern(e) => checker.check_extern(e),
            _ => {}
        }
    }
    for f in &results.mono_functions {
        checker.check_fn(f);
    }
    checker.moved_uses
}

#[derive(Clone)]
struct Loan {
    local: LocalId,
    mutable: bool,
    span: Span,
}

struct Own<'a> {
    results: &'a TypeckResults,
    diags: &'a mut DiagnosticEngine,
    moved: HashMap<LocalId, Span>,
    declared: HashSet<LocalId>,
    moved_uses: HashSet<NodeId>,
    stmt_loans: Vec<Loan>,
    block_loans: Vec<Vec<Loan>>,
    borrow_is_binding: bool,
}

impl<'a> Own<'a> {
    fn owned(&self, ty: &Type) -> bool {
        !is_copy(ty, self.results)
    }

    fn reset_fn(&mut self) {
        self.moved.clear();
        self.declared.clear();
        self.stmt_loans.clear();
        self.block_loans.clear();
        self.borrow_is_binding = false;
    }

    fn active_loans_on(&self, local: LocalId) -> Vec<Loan> {
        let mut v: Vec<Loan> = self.stmt_loans.iter().filter(|l| l.local == local).cloned().collect();
        for scope in &self.block_loans {
            for l in scope {
                if l.local == local {
                    v.push(l.clone());
                }
            }
        }
        v
    }

    fn place_root_local(&self, expr: &Expr) -> Option<LocalId> {
        match &expr.kind {
            ExprKind::Name(_) => self.local_of(expr),
            ExprKind::Field { base, .. } => self.place_root_local(base),
            ExprKind::Index { base, .. } => self.place_root_local(base),
            _ => None,
        }
    }

    fn local_of(&self, expr: &Expr) -> Option<LocalId> {
        if let ExprKind::Name(_) = &expr.kind {
            if let Some(Res::Local(id)) = self.results.name_res.get(&expr.id) {
                return Some(*id);
            }
        }
        None
    }

    fn check_extern(&mut self, e: &ExternDecl) {
        let Some((_, sig)) = self.results.func_by_name(&e.name.name) else { return };
        let param_tys: Vec<Type> = sig.params.iter().map(|(_, t)| t.clone()).collect();
        let ret_owned = self.owned(&sig.ret);
        for (p, ty) in e.params.iter().zip(param_tys.iter()) {
            if matches!(ty, Type::Ref { .. }) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_REF_IN_FFI,
                        "reference Lime tidak boleh melewati batas FFI",
                    )
                    .primary(p.span, "parameter ekstern bertipe reference"),
                );
            } else if self.owned(ty) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_OWNED_IN_FFI,
                        format!("tipe milik '{}' tidak boleh melewati batas FFI", ty.display()),
                    )
                    .primary(p.span, "parameter ekstern bertipe milik"),
                );
            }
        }
        if matches!(sig.ret, Type::Ref { .. }) {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_REF_IN_FFI,
                    "fungsi ekstern tidak boleh mengembalikan reference",
                )
                .primary(e.span, "nilai kembalian ekstern bertipe reference"),
            );
        } else if ret_owned {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_OWNED_IN_FFI,
                    "fungsi ekstern tidak boleh mengembalikan tipe milik",
                )
                .primary(e.span, "nilai kembalian ekstern bertipe milik"),
            );
        }
    }

    fn check_fn(&mut self, f: &Function) {
        self.reset_fn();
        for p in &f.params {
            if let Some(&id) = self.results.local_of_binding.get(&p.id) {
                self.declared.insert(id);
            }
        }
        self.check_block(&f.body);
    }

    fn check_block(&mut self, block: &Block) {
        self.block_loans.push(Vec::new());
        for stmt in &block.stmts {
            self.check_stmt(stmt);
            self.stmt_loans.clear();
        }
        self.block_loans.pop();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { init, id, .. } => {
                self.borrow_is_binding = is_borrow_call(init);
                self.visit(init, true);
                self.borrow_is_binding = false;
                if let Some(&local) = self.results.local_of_binding.get(id) {
                    self.declared.insert(local);
                    self.moved.remove(&local);
                }
            }
            Stmt::Assign { target, value, .. } => {
                self.visit(value, true);
                let tty = self.results.expr_type(target.id).clone();
                if self.owned(&tty) {
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_OWNED_REASSIGN,
                            "nilai bertipe milik belum bisa ditetapkan ulang",
                        )
                        .primary(target.span, "target bertipe milik")
                        .help("nilai bertipe milik dibuat sekali, lalu dipindahkan"),
                    );
                }
                if let ExprKind::Deref(inner) = &target.kind {
                    self.visit(inner, false);
                } else if let Some(root) = self.place_root_local(target) {
                    if !self.active_loans_on(root).is_empty() {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_MUTATE_WHILE_BORROWED,
                                "nilai ini masih dipakai oleh reference lain",
                            )
                            .primary(target.span, "nilai diubah di sini")
                            .secondary(self.active_loans_on(root)[0].span, "reference mulai dipakai di sini"),
                        );
                    }
                }
                if let ExprKind::Field { base, .. } = &target.kind {
                    self.visit(base, false);
                } else if let ExprKind::Index { base, index } = &target.kind {
                    self.visit(base, false);
                    self.visit(index, false);
                }
            }
            Stmt::Expr(e) => self.visit(e, false),
            Stmt::If(i) => self.check_if(i),
            Stmt::While { cond, body, .. } => {
                let before_declared = self.declared.clone();
                let before_moved = self.moved.clone();
                self.visit(cond, false);
                self.check_block(body);
                self.check_loop_moves(&before_moved, &before_declared);
            }
            Stmt::For(f) => {
                let before_declared = self.declared.clone();
                let before_moved = self.moved.clone();
                self.visit(&f.start, false);
                self.visit(&f.end, false);
                if let Some(&id) = self.results.local_of_binding.get(&f.var_id) {
                    self.declared.insert(id);
                }
                self.check_block(&f.body);
                self.check_loop_moves(&before_moved, &before_declared);
            }
            Stmt::ForEach(f) => {
                let before_declared = self.declared.clone();
                let before_moved = self.moved.clone();
                self.visit(&f.iterable, false);
                if let Some(&id) = self.results.local_of_binding.get(&f.var_id) {
                    self.declared.insert(id);
                }
                self.check_block(&f.body);
                self.check_loop_moves(&before_moved, &before_declared);
            }
            Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    let ty = self.results.expr_type(e.id).clone();
                    if matches!(ty, Type::Ref { .. }) {
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_REF_ESCAPE,
                                "reference tidak boleh dikembalikan dari fungsi",
                            )
                            .primary(e.span, "reference ini bisa menggantung")
                            .help("kembalikan nilainya, bukan reference-nya"),
                        );
                    }
                    self.visit(e, true);
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Block(b) => self.check_block(b),
        }
    }

    fn check_loop_moves(&mut self, before_moved: &HashMap<LocalId, Span>, before_declared: &HashSet<LocalId>) {
        let mut offenders = Vec::new();
        for (local, span) in &self.moved {
            if !before_moved.contains_key(local) && before_declared.contains(local) {
                offenders.push((*local, *span));
            }
        }
        for (_, span) in offenders {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_MOVE_IN_LOOP,
                    "nilai dari luar perulangan dipindahkan di dalam perulangan",
                )
                .primary(span, "dipindahkan di sini tiap iterasi")
                .help("nilai bertipe milik hanya bisa dipindahkan sekali"),
            );
        }
    }

    fn check_if(&mut self, i: &IfStmt) {
        self.visit(&i.cond, false);
        let before = self.moved.clone();

        let mut arm_new_moves: Vec<HashSet<LocalId>> = Vec::new();

        self.moved = before.clone();
        self.check_block(&i.then_block);
        arm_new_moves.push(self.new_moves(&before));

        for (cond, block) in &i.else_ifs {
            self.moved = before.clone();
            self.visit(cond, false);
            self.check_block(block);
            arm_new_moves.push(self.new_moves(&before));
        }

        if let Some(block) = &i.else_block {
            self.moved = before.clone();
            self.check_block(block);
            arm_new_moves.push(self.new_moves(&before));
        } else {
            arm_new_moves.push(HashSet::new());
        }

        let first = &arm_new_moves[0];
        let consistent = arm_new_moves.iter().all(|s| s == first);
        if !consistent {
            let mut all: HashSet<LocalId> = HashSet::new();
            for s in &arm_new_moves {
                all.extend(s.iter().copied());
            }
            for local in &all {
                if !arm_new_moves.iter().all(|s| s.contains(local)) {
                    let span = self
                        .moved
                        .get(local)
                        .copied()
                        .unwrap_or(i.span);
                    self.diags.emit(
                        Diagnostic::error(
                            codes::E_INCONSISTENT_MOVE,
                            "nilai hanya dipindahkan di sebagian cabang",
                        )
                        .primary(span, "dipindahkan di cabang ini")
                        .secondary(i.span, "cabang lain tidak memindahkannya")
                        .help("pindahkan nilai di semua cabang atau tidak sama sekali"),
                    );
                }
            }
        }

        self.moved = before.clone();
        for local in first {
            self.moved.entry(*local).or_insert(i.span);
        }
    }

    fn new_moves(&self, before: &HashMap<LocalId, Span>) -> HashSet<LocalId> {
        self.moved
            .keys()
            .filter(|k| !before.contains_key(k) && self.declared.contains(k))
            .copied()
            .collect()
    }

    fn visit(&mut self, expr: &Expr, consumed: bool) {
        match &expr.kind {
            ExprKind::Name(_) => {
                if let Some(local) = self.local_of(expr) {
                    let loans = self.active_loans_on(local);
                    let conflict = loans.iter().find(|l| l.mutable || consumed);
                    if let Some(l) = conflict {
                        let span = l.span;
                        self.diags.emit(
                            Diagnostic::error(
                                codes::E_BORROW_CONFLICT,
                                "nilai ini masih dipakai oleh reference lain",
                            )
                            .primary(expr.span, "nilai digunakan di sini")
                            .secondary(span, "reference mulai dipakai di sini"),
                        );
                    }
                    let ty = self.results.locals[&local].ty.clone();
                    if self.owned(&ty) {
                        if let Some(prev) = self.moved.get(&local).copied() {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_USE_AFTER_MOVE,
                                    "nilai ini sudah dipindahkan",
                                )
                                .primary(expr.span, "dipakai lagi di sini")
                                .secondary(prev, "dipindahkan di sini"),
                            );
                        } else if consumed {
                            self.moved.insert(local, expr.span);
                            self.moved_uses.insert(expr.id);
                        }
                    }
                }
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                self.visit(lhs, false);
                self.visit(rhs, false);
            }
            ExprKind::Unary { operand, .. } => self.visit(operand, false),
            ExprKind::Call { callee, args } => self.visit_call(expr, callee, args),
            ExprKind::Field { base, .. } => self.visit(base, false),
            ExprKind::Index { base, index } => {
                self.visit(base, false);
                self.visit(index, false);
            }
            ExprKind::StructLit { fields, .. } => {
                for f in fields {
                    self.reject_ref_escape(&f.value, "field struct");
                    self.visit(&f.value, true);
                }
            }
            ExprKind::ArrayLit(elems) => {
                for e in elems {
                    self.reject_ref_escape(e, "elemen larik");
                    self.visit(e, true);
                }
            }
            ExprKind::Deref(inner) => self.visit(inner, false),
            ExprKind::Try(inner) => self.visit(inner, false),
            ExprKind::Int { .. }
            | ExprKind::Float { .. }
            | ExprKind::Str(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_) => {}
        }
    }

    fn visit_call(&mut self, call: &Expr, callee: &Expr, args: &[Expr]) {
        if let Some(m) = self.results.method_calls.get(&call.id).cloned() {
            if let ExprKind::Field { base, .. } = &callee.kind {
                let base_is_ref = matches!(self.results.expr_type(base.id), Type::Ref { .. });
                match m.recv_mode {
                    RecvMode::Value => self.visit(base, true),
                    RecvMode::Ref => {
                        if base_is_ref {
                            self.visit(base, false);
                        } else {
                            self.handle_borrow(base, false);
                        }
                    }
                    RecvMode::RefMut => {
                        if base_is_ref {
                            self.visit(base, false);
                        } else {
                            self.handle_borrow(base, true);
                        }
                    }
                }
            }
            for a in args {
                self.visit(a, true);
            }
            return;
        }

        let res = self.results.name_res.get(&callee.id).cloned();

        if matches!(res, Some(Res::Builtin(Builtin::Ambil))) {
            self.handle_borrow(&args[0], false);
            return;
        }
        if matches!(res, Some(Res::Builtin(Builtin::AmbilUbah))) {
            self.handle_borrow(&args[0], true);
            return;
        }

        let is_extern = matches!(&res, Some(Res::Func(idx)) if self.results.functions[*idx].is_extern);
        let is_kotak = matches!(res, Some(Res::Builtin(Builtin::Kotak)));
        let borrow_only = matches!(
            res,
            Some(Res::Builtin(Builtin::Isi))
                | Some(Res::Builtin(Builtin::Panjang))
                | Some(Res::Builtin(Builtin::Tampilkan))
        );
        for a in args {
            let at = self.results.expr_type(a.id).clone();
            if is_extern && self.owned(&at) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_OWNED_IN_FFI,
                        format!("nilai bertipe milik '{}' tidak boleh melewati batas FFI", at.display()),
                    )
                    .primary(a.span, "argumen ekstern bertipe milik"),
                );
            }
            if is_extern && matches!(at, Type::Ref { .. }) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_REF_IN_FFI,
                        "reference Lime tidak boleh melewati batas FFI",
                    )
                    .primary(a.span, "argumen ekstern bertipe reference"),
                );
            }
            if is_kotak {
                self.reject_ref_escape(a, "kotak");
            }
            self.visit(a, !borrow_only);
        }
    }

    fn handle_borrow(&mut self, target: &Expr, mutable: bool) {
        for sub in index_exprs(target) {
            self.visit(sub, false);
        }
        let Some(root) = self.place_root_local(target) else { return };

        if let Some(prev) = self.moved.get(&root).copied() {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_USE_AFTER_MOVE,
                    "nilai ini sudah dipindahkan, tidak bisa diambil reference-nya",
                )
                .primary(target.span, "reference diambil di sini")
                .secondary(prev, "dipindahkan di sini"),
            );
            return;
        }

        let existing = self.active_loans_on(root);
        let conflict = existing.iter().find(|l| mutable || l.mutable);
        if let Some(l) = conflict {
            let span = l.span;
            self.diags.emit(
                Diagnostic::error(
                    codes::E_BORROW_CONFLICT,
                    if mutable {
                        "nilai ini masih dipakai reference lain, tidak bisa diambil reference mutable"
                    } else {
                        "nilai ini sudah dipakai reference mutable"
                    },
                )
                .primary(target.span, "reference baru diambil di sini")
                .secondary(span, "reference mulai dipakai di sini"),
            );
        }

        let loan = Loan { local: root, mutable, span: target.span };
        if self.borrow_is_binding {
            if let Some(scope) = self.block_loans.last_mut() {
                scope.push(loan);
            } else {
                self.stmt_loans.push(loan);
            }
        } else {
            self.stmt_loans.push(loan);
        }
    }

    fn reject_ref_escape(&mut self, e: &Expr, context: &str) {
        if matches!(self.results.expr_type(e.id), Type::Ref { .. }) {
            self.diags.emit(
                Diagnostic::error(
                    codes::E_REF_ESCAPE,
                    format!("reference tidak boleh disimpan di {}", context),
                )
                .primary(e.span, "reference bisa hidup lebih lama dari nilainya")
                .help("simpan nilainya, bukan reference-nya"),
            );
        }
    }
}

fn is_borrow_call(e: &Expr) -> bool {
    matches!(&e.kind, ExprKind::Call { callee, .. } if matches!(&callee.kind, ExprKind::Name(n) if n == "ambil" || n == "ambil_ubah"))
}

fn index_exprs(target: &Expr) -> Vec<&Expr> {
    let mut v = Vec::new();
    let mut node = target;
    loop {
        match &node.kind {
            ExprKind::Field { base, .. } => node = base,
            ExprKind::Index { base, index } => {
                v.push(index.as_ref());
                node = base;
            }
            _ => break,
        }
    }
    v
}
