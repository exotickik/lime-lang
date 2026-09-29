use crate::results::{Builtin, LocalId, Res, TypeckResults};
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
        _ => true,
    }
}

pub fn check_program(program: &Program, results: &TypeckResults, diags: &mut DiagnosticEngine) -> HashSet<NodeId> {
    let mut checker = Own { results, diags, moved: HashMap::new(), declared: HashSet::new(), moved_uses: HashSet::new() };
    for item in &program.items {
        match item {
            Item::Function(f) if f.type_params.is_empty() => checker.check_fn(f),
            Item::Main(m) => checker.check_block(&m.body),
            Item::Extern(e) => checker.check_extern(e),
            _ => {}
        }
    }
    for f in &results.mono_functions {
        checker.check_fn(f);
    }
    checker.moved_uses
}

struct Own<'a> {
    results: &'a TypeckResults,
    diags: &'a mut DiagnosticEngine,
    moved: HashMap<LocalId, Span>,
    declared: HashSet<LocalId>,
    moved_uses: HashSet<NodeId>,
}

impl<'a> Own<'a> {
    fn owned(&self, ty: &Type) -> bool {
        !is_copy(ty, self.results)
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
            if self.owned(ty) {
                self.diags.emit(
                    Diagnostic::error(
                        codes::E_OWNED_IN_FFI,
                        format!("tipe milik '{}' tidak boleh melewati batas FFI", ty.display()),
                    )
                    .primary(p.span, "parameter ekstern bertipe milik"),
                );
            }
        }
        if ret_owned {
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
        self.moved.clear();
        self.declared.clear();
        for p in &f.params {
            if let Some(&id) = self.results.local_of_binding.get(&p.id) {
                self.declared.insert(id);
            }
        }
        self.check_block(&f.body);
    }

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { init, id, .. } => {
                self.visit(init, true);
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
                            "penetapan ulang nilai bertipe milik belum didukung",
                        )
                        .primary(target.span, "target bertipe milik")
                        .help("nilai bertipe milik dibuat sekali lalu dipindahkan"),
                    );
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
                    "nilai yang dideklarasikan di luar perulangan dipindahkan di dalamnya",
                )
                .primary(span, "dipindahkan di sini pada setiap iterasi")
                .help("nilai bertipe milik hanya dapat dipindahkan sekali"),
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
                            "nilai dipindahkan pada sebagian cabang saja",
                        )
                        .primary(span, "dipindahkan di cabang ini")
                        .secondary(i.span, "cabang lain tidak memindahkannya")
                        .help("pindahkan nilai pada semua cabang atau tidak sama sekali"),
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
                    let ty = self.results.locals[&local].ty.clone();
                    if self.owned(&ty) {
                        if let Some(prev) = self.moved.get(&local).copied() {
                            self.diags.emit(
                                Diagnostic::error(
                                    codes::E_USE_AFTER_MOVE,
                                    "nilai digunakan setelah dipindahkan",
                                )
                                .primary(expr.span, "digunakan di sini setelah dipindahkan")
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
            ExprKind::Call { callee, args } => self.visit_call(callee, args),
            ExprKind::Field { base, .. } => self.visit(base, false),
            ExprKind::Index { base, index } => {
                self.visit(base, false);
                self.visit(index, false);
            }
            ExprKind::StructLit { fields, .. } => {
                for f in fields {
                    self.visit(&f.value, true);
                }
            }
            ExprKind::ArrayLit(elems) => {
                for e in elems {
                    self.visit(e, true);
                }
            }
            ExprKind::Int { .. }
            | ExprKind::Float { .. }
            | ExprKind::Str(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_) => {}
        }
    }

    fn visit_call(&mut self, callee: &Expr, args: &[Expr]) {
        let res = self.results.name_res.get(&callee.id).cloned();
        let is_extern = matches!(&res, Some(Res::Func(idx)) if self.results.functions[*idx].is_extern);
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
            self.visit(a, !borrow_only);
        }
    }
}
