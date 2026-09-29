use crate::ir::*;
use lime_ast::{
    BinOp, Block as AstBlock, Expr, ExprKind, ForStmt, Function, Ident, IfStmt, Item, MainBlock,
    Program, Stmt, UnOp,
};
use lime_diagnostics::{codes, Diagnostic, DiagnosticEngine};
use lime_sema::{Builtin, Res, TypeckResults};
use lime_types::Type;
use std::collections::HashMap;

pub fn lower(program: &Program, results: &TypeckResults, diags: &mut DiagnosticEngine) -> Module {
    let mut module = Module { name: program.name.name.clone(), ..Module::default() };

    for info in &results.structs {
        module.structs.push(IrStruct {
            name: info.name.clone(),
            fields: info.fields.iter().map(|f| f.ty.clone()).collect(),
        });
    }

    for info in &results.enums {
        module.enums.push(IrEnum { name: info.name.clone(), variants: info.variants.clone() });
    }

    for sig in &results.functions {
        if sig.is_extern {
            module.externs.push(ExternSig {
                name: sig.name.clone(),
                params: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                variadic: sig.variadic,
                ret: sig.ret.clone(),
            });
        }
    }

    for item in &program.items {
        match item {
            Item::Function(f) => {
                let func = FnLowerer::new(&mut module, results, diags).lower_fn(f);
                module.funcs.push(func);
            }
            Item::Main(m) => {
                let func = FnLowerer::new(&mut module, results, diags).lower_main(m);
                module.funcs.push(func);
            }
            _ => {}
        }
    }

    module
}

struct FnLowerer<'a> {
    module: &'a mut Module,
    results: &'a TypeckResults,
    diags: &'a mut DiagnosticEngine,
    slots: Vec<Slot>,
    blocks: Vec<Block>,
    current: BlockId,
    next_value: u32,
    slot_of_local: HashMap<u32, SlotId>,
    loop_stack: Vec<(BlockId, BlockId)>,
    ret_ty: Type,
    is_main: bool,
}

impl<'a> FnLowerer<'a> {
    fn new(
        module: &'a mut Module,
        results: &'a TypeckResults,
        diags: &'a mut DiagnosticEngine,
    ) -> FnLowerer<'a> {
        FnLowerer {
            module,
            results,
            diags,
            slots: Vec::new(),
            blocks: Vec::new(),
            current: BlockId(0),
            next_value: 0,
            slot_of_local: HashMap::new(),
            loop_stack: Vec::new(),
            ret_ty: Type::Unit,
            is_main: false,
        }
    }

    fn lower_fn(mut self, f: &Function) -> Func {
        let (_, sig) = self.results.func_by_name(&f.name.name).unwrap();
        let sig = sig.clone();
        self.ret_ty = sig.ret.clone();
        let entry = self.new_block();
        self.current = entry;

        let mut params = Vec::new();
        for (p, (_, pty)) in f.params.iter().zip(sig.params.iter()) {
            let val = self.fresh_value();
            params.push((val, pty.clone()));
            let local = self.results.local_of_binding[&p.name.span];
            let slot = self.new_slot(pty.clone());
            self.slot_of_local.insert(local.0, slot);
            self.emit(None, InstrKind::Store { slot, value: Operand::Val(val) });
        }

        self.lower_block(&f.body);
        self.finish_fallthrough();

        Func {
            name: f.name.name.clone(),
            params,
            ret: sig.ret.clone(),
            slots: self.slots,
            blocks: self.blocks,
            entry,
            is_main: false,
        }
    }

    fn lower_main(mut self, m: &MainBlock) -> Func {
        self.is_main = true;
        self.ret_ty = Type::I64;
        let entry = self.new_block();
        self.current = entry;
        self.lower_block(&m.body);
        self.finish_fallthrough();
        Func {
            name: "main".to_string(),
            params: Vec::new(),
            ret: Type::I32,
            slots: self.slots,
            blocks: self.blocks,
            entry,
            is_main: true,
        }
    }

    fn finish_fallthrough(&mut self) {
        if self.block(self.current).term.is_none() {
            if self.is_main {
                self.set_term(Terminator::Ret(Some(Operand::ConstInt(0, Type::I32))));
            } else if self.ret_ty == Type::Unit {
                self.set_term(Terminator::Ret(None));
            } else {
                self.set_term(Terminator::Unreachable);
            }
        }
    }

    fn lower_block(&mut self, block: &AstBlock) {
        for stmt in &block.stmts {
            if self.block(self.current).term.is_some() {
                let dead = self.new_block();
                self.current = dead;
            }
            self.lower_stmt(stmt);
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, init, .. } => {
                let local = self.results.local_of_binding[&name.span];
                let ty = self.results.locals[&local].ty.clone();
                let op = self.lower_expr(init);
                let slot = self.new_slot(ty);
                self.slot_of_local.insert(local.0, slot);
                self.emit(None, InstrKind::Store { slot, value: op });
            }
            Stmt::Assign { target, value, .. } => match &target.kind {
                ExprKind::Name(_) => {
                    if let Some(Res::Local(id)) = self.results.name_res.get(&target.id) {
                        let slot = self.slot_of_local[&id.0];
                        let op = self.lower_expr(value);
                        self.emit(None, InstrKind::Store { slot, value: op });
                    }
                }
                ExprKind::Field { .. } => self.lower_field_assign(target, value),
                ExprKind::Index { .. } => self.lower_indexed_assign(target, value),
                _ => self.unsupported(target.span, "target penugasan tidak didukung backend"),
            },
            Stmt::Expr(e) => {
                self.lower_expr(e);
            }
            Stmt::If(if_stmt) => self.lower_if(if_stmt),
            Stmt::While { cond, body, .. } => self.lower_while(cond, body),
            Stmt::For(for_stmt) => self.lower_for(for_stmt),
            Stmt::ForEach(fe) => self.lower_foreach(fe),
            Stmt::Return { value, .. } => {
                let op = match value {
                    Some(e) => {
                        let mut o = self.lower_expr(e);
                        if self.is_main {
                            o = self.cast(o, Type::I64, Type::I32);
                        }
                        Some(o)
                    }
                    None => {
                        if self.is_main {
                            Some(Operand::ConstInt(0, Type::I32))
                        } else if self.ret_ty == Type::Unit {
                            None
                        } else {
                            Some(self.zero_of(&self.ret_ty.clone()))
                        }
                    }
                };
                self.set_term(Terminator::Ret(op));
            }
            Stmt::Break(_) => {
                if let Some(&(_, brk)) = self.loop_stack.last() {
                    self.set_term(Terminator::Br(brk));
                }
            }
            Stmt::Continue(_) => {
                if let Some(&(cont, _)) = self.loop_stack.last() {
                    self.set_term(Terminator::Br(cont));
                }
            }
            Stmt::Block(b) => self.lower_block(b),
        }
    }

    fn lower_if(&mut self, if_stmt: &IfStmt) {
        let end_bb = self.new_block();
        self.lower_if_chain(&if_stmt.cond, &if_stmt.then_block, &if_stmt.else_ifs, &if_stmt.else_block, 0, end_bb);
        self.current = end_bb;
    }

    fn lower_if_chain(
        &mut self,
        cond: &Expr,
        then_block: &AstBlock,
        else_ifs: &[(Expr, AstBlock)],
        else_block: &Option<AstBlock>,
        idx: usize,
        end_bb: BlockId,
    ) {
        let cond_op = self.lower_expr(cond);
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        self.set_term(Terminator::CondBr { cond: cond_op, then_bb, else_bb });

        self.current = then_bb;
        self.lower_block(then_block);
        if self.block(self.current).term.is_none() {
            self.set_term(Terminator::Br(end_bb));
        }

        self.current = else_bb;
        if idx < else_ifs.len() {
            let (c, b) = &else_ifs[idx];
            self.lower_if_chain(c, b, else_ifs, else_block, idx + 1, end_bb);
        } else if let Some(b) = else_block {
            self.lower_block(b);
            if self.block(self.current).term.is_none() {
                self.set_term(Terminator::Br(end_bb));
            }
        } else {
            self.set_term(Terminator::Br(end_bb));
        }
    }

    fn lower_while(&mut self, cond: &Expr, body: &AstBlock) {
        let head = self.new_block();
        let body_bb = self.new_block();
        let end_bb = self.new_block();
        self.set_term(Terminator::Br(head));

        self.current = head;
        let c = self.lower_expr(cond);
        self.set_term(Terminator::CondBr { cond: c, then_bb: body_bb, else_bb: end_bb });

        self.current = body_bb;
        self.loop_stack.push((head, end_bb));
        self.lower_block(body);
        self.loop_stack.pop();
        if self.block(self.current).term.is_none() {
            self.set_term(Terminator::Br(head));
        }

        self.current = end_bb;
    }

    fn lower_for(&mut self, for_stmt: &ForStmt) {
        let local = self.results.local_of_binding[&for_stmt.var.span];
        let ty = self.results.locals[&local].ty.clone();
        let signed = ty.is_signed_int();

        let start = self.lower_expr(&for_stmt.start);
        let slot = self.new_slot(ty.clone());
        self.slot_of_local.insert(local.0, slot);
        self.emit(None, InstrKind::Store { slot, value: start });

        let head = self.new_block();
        let body_bb = self.new_block();
        let step_bb = self.new_block();
        let end_bb = self.new_block();
        self.set_term(Terminator::Br(head));

        self.current = head;
        let cur = self.load_slot(slot, ty.clone());
        let end_val = self.lower_expr(&for_stmt.end);
        let pred = if for_stmt.inclusive { CmpPred::Le } else { CmpPred::Lt };
        let cmp = self.fresh_value();
        self.emit(
            Some(cmp),
            InstrKind::Cmp { pred, ty: ty.clone(), signed, lhs: Operand::Val(cur), rhs: end_val },
        );
        self.set_term(Terminator::CondBr {
            cond: Operand::Val(cmp),
            then_bb: body_bb,
            else_bb: end_bb,
        });

        self.current = body_bb;
        self.loop_stack.push((step_bb, end_bb));
        self.lower_block(&for_stmt.body);
        self.loop_stack.pop();
        if self.block(self.current).term.is_none() {
            self.set_term(Terminator::Br(step_bb));
        }

        self.current = step_bb;
        let cur2 = self.load_slot(slot, ty.clone());
        let next = self.fresh_value();
        self.emit(
            Some(next),
            InstrKind::Bin {
                op: ArithOp::Add,
                ty: ty.clone(),
                signed,
                lhs: Operand::Val(cur2),
                rhs: Operand::ConstInt(1, ty.clone()),
            },
        );
        self.emit(None, InstrKind::Store { slot, value: Operand::Val(next) });
        self.set_term(Terminator::Br(head));

        self.current = end_bb;
    }

    fn lower_expr(&mut self, expr: &Expr) -> Operand {
        match &expr.kind {
            ExprKind::Int { value, .. } => {
                let ty = self.results.expr_type(expr.id).clone();
                match ty {
                    Type::Float(_) => Operand::ConstFloat(*value as f64, ty),
                    _ => Operand::ConstInt(*value, ty),
                }
            }
            ExprKind::Float { value, .. } => {
                let ty = self.results.expr_type(expr.id).clone();
                Operand::ConstFloat(*value, ty)
            }
            ExprKind::Bool(b) => Operand::ConstBool(*b),
            ExprKind::Char(c) => Operand::ConstInt(*c as u32 as u128, Type::Char),
            ExprKind::Str(s) => {
                let id = self.module.intern_str(s.clone().into_bytes());
                Operand::Str(id)
            }
            ExprKind::Name(_) => match self.results.name_res.get(&expr.id) {
                Some(Res::Local(id)) => {
                    let slot = self.slot_of_local[&id.0];
                    let ty = self.results.locals[id].ty.clone();
                    Operand::Val(self.load_slot(slot, ty))
                }
                _ => {
                    self.unsupported(expr.span, "referensi nilai ini belum didukung backend");
                    Operand::ConstInt(0, Type::I64)
                }
            },
            ExprKind::Binary { op, lhs, rhs } => self.lower_binary(*op, lhs, rhs, expr),
            ExprKind::Unary { op, operand } => self.lower_unary(*op, operand, expr),
            ExprKind::Call { callee, args } => self.lower_call(callee, args, expr),
            ExprKind::Field { base, field } => {
                if let Some(&(eid, tag)) = self.results.enum_variant.get(&expr.id) {
                    Operand::ConstInt(tag as u128, Type::Enum(eid))
                } else {
                    self.lower_field_read(base, field, expr)
                }
            }
            ExprKind::StructLit { .. } => self.lower_struct_lit(expr),
            ExprKind::ArrayLit(elems) => self.lower_array_lit(elems, expr),
            ExprKind::Index { base, index } => self.lower_index_read(base, index, expr),
        }
    }

    fn lower_array_lit(&mut self, elems: &[Expr], expr: &Expr) -> Operand {
        let ty = self.results.expr_type(expr.id).clone();
        let elem_ty = match &ty {
            Type::Array { elem, .. } => (**elem).clone(),
            _ => Type::I64,
        };
        let ops: Vec<Operand> = elems.iter().map(|e| self.lower_expr(e)).collect();
        let dst = self.fresh_value();
        self.emit(Some(dst), InstrKind::MakeArray { elem_ty, elems: ops });
        Operand::Val(dst)
    }

    fn lower_index_read(&mut self, base: &Expr, index: &Expr, expr: &Expr) -> Operand {
        let elem_ty = self.results.expr_type(expr.id).clone();
        let ptr = self.elem_ptr(base, index);
        let dst = self.fresh_value();
        self.emit(Some(dst), InstrKind::LoadPtr { ptr, ty: elem_ty });
        Operand::Val(dst)
    }

    fn lower_indexed_assign(&mut self, target: &Expr, value: &Expr) {
        let ExprKind::Index { base, index } = &target.kind else {
            return;
        };
        let elem_ty = self.results.expr_type(target.id).clone();
        let val = self.lower_expr(value);
        let ptr = self.elem_ptr(base, index);
        self.emit(None, InstrKind::StorePtr { ptr, ty: elem_ty, value: val });
    }

    fn elem_ptr(&mut self, base: &Expr, index: &Expr) -> Operand {
        let base_ty = self.results.expr_type(base.id).clone();
        let size = match &base_ty {
            Type::Array { size, .. } => *size,
            _ => 0,
        };
        if let Some((slot, mut path)) = self.resolve_place(base) {
            let idx = self.lower_index_value(index, size);
            path.push(GepIndex::ArrayVal(idx));
            let agg_ty = self.slots[slot.0 as usize].ty.clone();
            let dst = self.fresh_value();
            self.emit(Some(dst), InstrKind::Gep { slot, agg_ty, indices: path });
            Operand::Val(dst)
        } else {
            let arr = self.lower_expr(base);
            let tmp = self.new_slot(base_ty.clone());
            self.emit(None, InstrKind::Store { slot: tmp, value: arr });
            let idx = self.lower_index_value(index, size);
            let dst = self.fresh_value();
            self.emit(
                Some(dst),
                InstrKind::Gep {
                    slot: tmp,
                    agg_ty: base_ty,
                    indices: vec![GepIndex::ArrayVal(idx)],
                },
            );
            Operand::Val(dst)
        }
    }

    fn lower_index_value(&mut self, index: &Expr, size: u64) -> Operand {
        let idx_ty = self.results.expr_type(index.id).clone();
        let raw = self.lower_expr(index);
        let idx = self.cast(raw, idx_ty, Type::I64);
        self.bounds_check(idx.clone(), size);
        idx
    }

    fn bounds_check(&mut self, idx: Operand, len: u64) {
        self.module.needs_abort = true;
        let fail = self.new_block();
        let cont = self.new_block();
        let ok = self.new_block();

        let lo = self.fresh_value();
        self.emit(
            Some(lo),
            InstrKind::Cmp {
                pred: CmpPred::Lt,
                ty: Type::I64,
                signed: true,
                lhs: idx.clone(),
                rhs: Operand::ConstInt(0, Type::I64),
            },
        );
        self.set_term(Terminator::CondBr { cond: Operand::Val(lo), then_bb: fail, else_bb: cont });

        self.current = cont;
        let hi = self.fresh_value();
        self.emit(
            Some(hi),
            InstrKind::Cmp {
                pred: CmpPred::Ge,
                ty: Type::I64,
                signed: true,
                lhs: idx,
                rhs: Operand::ConstInt(len as u128, Type::I64),
            },
        );
        self.set_term(Terminator::CondBr { cond: Operand::Val(hi), then_bb: fail, else_bb: ok });

        self.current = fail;
        self.emit(
            None,
            InstrKind::Call {
                callee: "abort".to_string(),
                args: Vec::new(),
                ret: Type::Unit,
                variadic_from: None,
            },
        );
        self.set_term(Terminator::Unreachable);

        self.current = ok;
    }

    fn resolve_place(&mut self, expr: &Expr) -> Option<(SlotId, Vec<GepIndex>)> {
        match &expr.kind {
            ExprKind::Name(_) => match self.results.name_res.get(&expr.id) {
                Some(Res::Local(id)) => Some((self.slot_of_local[&id.0], Vec::new())),
                _ => None,
            },
            ExprKind::Field { base, field } => {
                let (slot, mut path) = self.resolve_place(base)?;
                let base_ty = self.results.expr_type(base.id).clone();
                if let Type::Struct(sid) = &base_ty {
                    path.push(GepIndex::Field(self.field_index(*sid, &field.name)));
                    Some((slot, path))
                } else {
                    None
                }
            }
            ExprKind::Index { base, index } => {
                let base_ty = self.results.expr_type(base.id).clone();
                let size = match &base_ty {
                    Type::Array { size, .. } => *size,
                    _ => return None,
                };
                let (slot, mut path) = self.resolve_place(base)?;
                let idx = self.lower_index_value(index, size);
                path.push(GepIndex::ArrayVal(idx));
                Some((slot, path))
            }
            _ => None,
        }
    }

    fn lower_foreach(&mut self, fe: &lime_ast::ForEachStmt) {
        let arr_ty = self.results.expr_type(fe.iterable.id).clone();
        let (elem_ty, size) = match &arr_ty {
            Type::Array { elem, size } => ((**elem).clone(), *size),
            _ => return,
        };
        let arr_val = self.lower_expr(&fe.iterable);
        let arr_slot = self.new_slot(arr_ty.clone());
        self.emit(None, InstrKind::Store { slot: arr_slot, value: arr_val });

        let idx_slot = self.new_slot(Type::I64);
        self.emit(None, InstrKind::Store { slot: idx_slot, value: Operand::ConstInt(0, Type::I64) });

        let local = self.results.local_of_binding[&fe.var.span];
        let var_slot = self.new_slot(elem_ty.clone());
        self.slot_of_local.insert(local.0, var_slot);

        let head = self.new_block();
        let body_bb = self.new_block();
        let step_bb = self.new_block();
        let end_bb = self.new_block();
        self.set_term(Terminator::Br(head));

        self.current = head;
        let i = self.load_slot(idx_slot, Type::I64);
        let cmp = self.fresh_value();
        self.emit(
            Some(cmp),
            InstrKind::Cmp {
                pred: CmpPred::Lt,
                ty: Type::I64,
                signed: true,
                lhs: Operand::Val(i),
                rhs: Operand::ConstInt(size as u128, Type::I64),
            },
        );
        self.set_term(Terminator::CondBr { cond: Operand::Val(cmp), then_bb: body_bb, else_bb: end_bb });

        self.current = body_bb;
        let i_body = self.load_slot(idx_slot, Type::I64);
        let elem_ptr = self.fresh_value();
        self.emit(
            Some(elem_ptr),
            InstrKind::Gep {
                slot: arr_slot,
                agg_ty: arr_ty.clone(),
                indices: vec![GepIndex::ArrayVal(Operand::Val(i_body))],
            },
        );
        let elem = self.fresh_value();
        self.emit(Some(elem), InstrKind::LoadPtr { ptr: Operand::Val(elem_ptr), ty: elem_ty });
        self.emit(None, InstrKind::Store { slot: var_slot, value: Operand::Val(elem) });
        self.loop_stack.push((step_bb, end_bb));
        self.lower_block(&fe.body);
        self.loop_stack.pop();
        if self.block(self.current).term.is_none() {
            self.set_term(Terminator::Br(step_bb));
        }

        self.current = step_bb;
        let i2 = self.load_slot(idx_slot, Type::I64);
        let next = self.fresh_value();
        self.emit(
            Some(next),
            InstrKind::Bin {
                op: ArithOp::Add,
                ty: Type::I64,
                signed: true,
                lhs: Operand::Val(i2),
                rhs: Operand::ConstInt(1, Type::I64),
            },
        );
        self.emit(None, InstrKind::Store { slot: idx_slot, value: Operand::Val(next) });
        self.set_term(Terminator::Br(head));

        self.current = end_bb;
    }

    fn lower_struct_lit(&mut self, expr: &Expr) -> Operand {
        let ty = self.results.expr_type(expr.id).clone();
        let Type::Struct(sid) = ty else {
            return Operand::ConstInt(0, Type::I64);
        };
        let ExprKind::StructLit { fields, .. } = &expr.kind else {
            return Operand::ConstInt(0, Type::I64);
        };
        let info = &self.results.structs[sid.0 as usize];
        let order: Vec<(usize, Type)> =
            info.fields.iter().enumerate().map(|(i, f)| (i, f.ty.clone())).collect();
        let field_names: Vec<String> = info.fields.iter().map(|f| f.name.clone()).collect();

        let mut ordered_ops: Vec<Operand> = vec![Operand::Unit; order.len()];
        for lit in fields {
            if let Some(idx) = field_names.iter().position(|n| *n == lit.name.name) {
                ordered_ops[idx] = self.lower_expr(&lit.value);
            }
        }
        let dst = self.fresh_value();
        self.emit(Some(dst), InstrKind::MakeStruct { ty: ty.clone(), fields: ordered_ops });
        Operand::Val(dst)
    }

    fn lower_field_read(&mut self, base: &Expr, field: &Ident, expr: &Expr) -> Operand {
        let base_ty = self.results.expr_type(base.id).clone();
        let field_ty = self.results.expr_type(expr.id).clone();
        let Type::Struct(sid) = &base_ty else {
            self.unsupported(expr.span, "akses field pada nilai bukan struct");
            return Operand::ConstInt(0, Type::I64);
        };
        let index = self.field_index(*sid, &field.name);
        let agg = self.lower_expr(base);
        let dst = self.fresh_value();
        self.emit(
            Some(dst),
            InstrKind::ExtractField { agg, agg_ty: base_ty.clone(), index, field_ty },
        );
        Operand::Val(dst)
    }

    fn lower_field_assign(&mut self, target: &Expr, value: &Expr) {
        let mut indices = Vec::new();
        let mut node = target;
        let root_slot;
        loop {
            match &node.kind {
                ExprKind::Field { base, field } => {
                    let base_ty = self.results.expr_type(base.id).clone();
                    if let Type::Struct(sid) = &base_ty {
                        indices.push(self.field_index(*sid, &field.name));
                    } else {
                        self.unsupported(target.span, "target field pada nilai bukan struct");
                        return;
                    }
                    node = base;
                }
                ExprKind::Name(_) => {
                    match self.results.name_res.get(&node.id) {
                        Some(Res::Local(id)) => {
                            root_slot = self.slot_of_local[&id.0];
                        }
                        _ => {
                            self.unsupported(target.span, "akar penugasan field bukan variabel");
                            return;
                        }
                    }
                    break;
                }
                _ => {
                    self.unsupported(target.span, "target penugasan field tidak didukung");
                    return;
                }
            }
        }
        indices.reverse();
        let root_ty = self.slots[root_slot.0 as usize].ty.clone();
        let field_ty = self.results.expr_type(target.id).clone();
        let val = self.lower_expr(value);
        let agg = self.load_slot(root_slot, root_ty.clone());
        let updated = self.fresh_value();
        self.emit(
            Some(updated),
            InstrKind::InsertField {
                agg: Operand::Val(agg),
                agg_ty: root_ty,
                indices,
                value: val,
                field_ty,
            },
        );
        self.emit(None, InstrKind::Store { slot: root_slot, value: Operand::Val(updated) });
    }

    fn field_index(&self, sid: lime_types::StructId, name: &str) -> u32 {
        self.results.structs[sid.0 as usize]
            .fields
            .iter()
            .position(|f| f.name == name)
            .map(|i| i as u32)
            .unwrap_or(0)
    }

    fn lower_string_eq(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr) -> Operand {
        self.module.needs_strcmp = true;
        let l = self.lower_expr(lhs);
        let r = self.lower_expr(rhs);
        let cmpres = self.fresh_value();
        self.emit(
            Some(cmpres),
            InstrKind::Call {
                callee: "strcmp".to_string(),
                args: vec![l, r],
                ret: Type::I32,
                variadic_from: None,
            },
        );
        let pred = if op == BinOp::Eq { CmpPred::Eq } else { CmpPred::Ne };
        let dst = self.fresh_value();
        self.emit(
            Some(dst),
            InstrKind::Cmp {
                pred,
                ty: Type::I32,
                signed: true,
                lhs: Operand::Val(cmpres),
                rhs: Operand::ConstInt(0, Type::I32),
            },
        );
        Operand::Val(dst)
    }

    fn lower_binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, expr: &Expr) -> Operand {
        if op == BinOp::And || op == BinOp::Or {
            return self.lower_logical(op, lhs, rhs);
        }
        let operand_ty = self.results.expr_type(lhs.id).clone();
        if operand_ty == Type::Str && (op == BinOp::Eq || op == BinOp::Ne) {
            return self.lower_string_eq(op, lhs, rhs);
        }
        let signed = operand_ty.is_signed_int();
        let l = self.lower_expr(lhs);
        let r = self.lower_expr(rhs);
        let dst = self.fresh_value();
        if op.is_comparison() {
            let pred = match op {
                BinOp::Eq => CmpPred::Eq,
                BinOp::Ne => CmpPred::Ne,
                BinOp::Lt => CmpPred::Lt,
                BinOp::Le => CmpPred::Le,
                BinOp::Gt => CmpPred::Gt,
                BinOp::Ge => CmpPred::Ge,
                _ => unreachable!(),
            };
            self.emit(Some(dst), InstrKind::Cmp { pred, ty: operand_ty, signed, lhs: l, rhs: r });
        } else {
            let aop = match op {
                BinOp::Add => ArithOp::Add,
                BinOp::Sub => ArithOp::Sub,
                BinOp::Mul => ArithOp::Mul,
                BinOp::Div => ArithOp::Div,
                BinOp::Rem => ArithOp::Rem,
                _ => unreachable!(),
            };
            let result_ty = self.results.expr_type(expr.id).clone();
            self.emit(
                Some(dst),
                InstrKind::Bin { op: aop, ty: result_ty, signed, lhs: l, rhs: r },
            );
        }
        Operand::Val(dst)
    }

    fn lower_logical(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr) -> Operand {
        let slot = self.new_slot(Type::Bool);
        let l = self.lower_expr(lhs);
        let rhs_bb = self.new_block();
        let short_bb = self.new_block();
        let end_bb = self.new_block();

        if op == BinOp::And {
            self.set_term(Terminator::CondBr { cond: l, then_bb: rhs_bb, else_bb: short_bb });
            self.current = short_bb;
            self.emit(None, InstrKind::Store { slot, value: Operand::ConstBool(false) });
            self.set_term(Terminator::Br(end_bb));
        } else {
            self.set_term(Terminator::CondBr { cond: l, then_bb: short_bb, else_bb: rhs_bb });
            self.current = short_bb;
            self.emit(None, InstrKind::Store { slot, value: Operand::ConstBool(true) });
            self.set_term(Terminator::Br(end_bb));
        }

        self.current = rhs_bb;
        let r = self.lower_expr(rhs);
        self.emit(None, InstrKind::Store { slot, value: r });
        self.set_term(Terminator::Br(end_bb));

        self.current = end_bb;
        Operand::Val(self.load_slot(slot, Type::Bool))
    }

    fn lower_unary(&mut self, op: UnOp, operand: &Expr, expr: &Expr) -> Operand {
        let o = self.lower_expr(operand);
        let dst = self.fresh_value();
        match op {
            UnOp::Neg => {
                let ty = self.results.expr_type(expr.id).clone();
                self.emit(Some(dst), InstrKind::Neg { ty, operand: o });
            }
            UnOp::Not => {
                self.emit(Some(dst), InstrKind::Not { operand: o });
            }
        }
        Operand::Val(dst)
    }

    fn lower_call(&mut self, callee: &Expr, args: &[Expr], expr: &Expr) -> Operand {
        match self.results.name_res.get(&callee.id) {
            Some(Res::Builtin(Builtin::Tampilkan)) => {
                self.lower_print(&args[0]);
                Operand::Unit
            }
            Some(Res::Builtin(Builtin::Panjang)) => {
                let arg_ty = self.results.expr_type(args[0].id).clone();
                match arg_ty {
                    Type::Str => {
                        self.module.needs_strlen = true;
                        let s = self.lower_expr(&args[0]);
                        let dst = self.fresh_value();
                        self.emit(
                            Some(dst),
                            InstrKind::Call {
                                callee: "strlen".to_string(),
                                args: vec![s],
                                ret: Type::I64,
                                variadic_from: None,
                            },
                        );
                        Operand::Val(dst)
                    }
                    Type::Array { size, .. } => {
                        self.lower_expr(&args[0]);
                        Operand::ConstInt(size as u128, Type::I64)
                    }
                    _ => Operand::ConstInt(0, Type::I64),
                }
            }
            Some(Res::Func(idx)) => {
                let sig = self.results.functions[*idx].clone();
                let arg_ops: Vec<Operand> = args.iter().map(|a| self.lower_expr(a)).collect();
                let ret = sig.ret.clone();
                if ret == Type::Unit {
                    self.emit(
                        None,
                        InstrKind::Call {
                            callee: sig.name.clone(),
                            args: arg_ops,
                            ret: Type::Unit,
                            variadic_from: None,
                        },
                    );
                    Operand::Unit
                } else {
                    let dst = self.fresh_value();
                    self.emit(
                        Some(dst),
                        InstrKind::Call {
                            callee: sig.name.clone(),
                            args: arg_ops,
                            ret: ret.clone(),
                            variadic_from: None,
                        },
                    );
                    Operand::Val(dst)
                }
            }
            _ => {
                self.unsupported(expr.span, "pemanggilan ini belum didukung backend");
                Operand::ConstInt(0, Type::I64)
            }
        }
    }

    fn lower_print(&mut self, arg: &Expr) {
        self.module.needs_printf = true;
        let ty = self.results.expr_type(arg.id).clone();
        match ty {
            Type::Str => {
                let fmt = self.fmt_global("%s\n");
                let val = self.lower_expr(arg);
                self.call_printf(fmt, vec![val]);
            }
            Type::Bool => {
                let fmt = self.fmt_global("%s\n");
                let benar = self.module.intern_str(b"benar".to_vec());
                let salah = self.module.intern_str(b"salah".to_vec());
                let cond = self.lower_expr(arg);
                let sel = self.fresh_value();
                self.emit(
                    Some(sel),
                    InstrKind::Select {
                        cond,
                        then_val: Operand::Str(benar),
                        else_val: Operand::Str(salah),
                        ty: Type::Str,
                    },
                );
                self.call_printf(fmt, vec![Operand::Val(sel)]);
            }
            Type::Float(_) => {
                let fmt = self.fmt_global("%g\n");
                let val = self.lower_expr(arg);
                let promoted = self.cast(val, ty.clone(), Type::F64);
                self.call_printf(fmt, vec![promoted]);
            }
            Type::Int { signed, width } => {
                let val = self.lower_expr(arg);
                let (fmt_str, target) = match (signed, width.bits()) {
                    (true, 64) => ("%lld\n", Type::I64),
                    (false, 64) => ("%llu\n", Type::U64),
                    (true, _) => ("%d\n", Type::I32),
                    (false, _) => ("%u\n", Type::U32),
                };
                let promoted = self.cast(val, ty.clone(), target);
                let fmt = self.fmt_global(fmt_str);
                self.call_printf(fmt, vec![promoted]);
            }
            Type::Char => {
                let fmt = self.fmt_global("%c\n");
                let val = self.lower_expr(arg);
                self.call_printf(fmt, vec![val]);
            }
            Type::Enum(eid) => {
                let tag = self.lower_expr(arg);
                let name_op = self.enum_name_operand(eid, tag);
                let fmt = self.fmt_global("%s\n");
                self.call_printf(fmt, vec![name_op]);
            }
            _ => self.unsupported(arg.span, "tipe ini tidak dapat dicetak backend"),
        }
    }

    fn enum_name_operand(&mut self, eid: lime_types::EnumId, tag: Operand) -> Operand {
        let variants = self.module.enums[eid.0 as usize].variants.clone();
        let globals: Vec<GlobalId> =
            variants.iter().map(|v| self.module.intern_str(v.clone().into_bytes())).collect();
        let n = globals.len();
        let mut acc = Operand::Str(globals[n - 1]);
        for i in (0..n - 1).rev() {
            let cmp = self.fresh_value();
            self.emit(
                Some(cmp),
                InstrKind::Cmp {
                    pred: CmpPred::Eq,
                    ty: Type::Enum(eid),
                    signed: false,
                    lhs: tag.clone(),
                    rhs: Operand::ConstInt(i as u128, Type::Enum(eid)),
                },
            );
            let sel = self.fresh_value();
            self.emit(
                Some(sel),
                InstrKind::Select {
                    cond: Operand::Val(cmp),
                    then_val: Operand::Str(globals[i]),
                    else_val: acc,
                    ty: Type::Str,
                },
            );
            acc = Operand::Val(sel);
        }
        acc
    }

    fn call_printf(&mut self, fmt: GlobalId, mut extra: Vec<Operand>) {
        let mut args = vec![Operand::Str(fmt)];
        args.append(&mut extra);
        self.emit(
            None,
            InstrKind::Call {
                callee: "printf".to_string(),
                args,
                ret: Type::I32,
                variadic_from: Some(1),
            },
        );
    }

    fn fmt_global(&mut self, s: &str) -> GlobalId {
        self.module.intern_str(s.as_bytes().to_vec())
    }

    fn cast(&mut self, val: Operand, from: Type, to: Type) -> Operand {
        if from == to {
            return val;
        }
        let kind = match (&from, &to) {
            (Type::Int { width: fw, signed }, Type::Int { width: tw, .. }) => {
                if tw.bits() > fw.bits() {
                    if *signed {
                        CastKind::SExt
                    } else {
                        CastKind::ZExt
                    }
                } else if tw.bits() < fw.bits() {
                    CastKind::Trunc
                } else {
                    return val;
                }
            }
            (Type::Char, Type::Int { width, .. }) => {
                if width.bits() > 32 {
                    CastKind::ZExt
                } else if width.bits() < 32 {
                    CastKind::Trunc
                } else {
                    return val;
                }
            }
            (Type::Float(fw), Type::Float(tw)) => {
                if tw.bits() > fw.bits() {
                    CastKind::FpExt
                } else {
                    CastKind::FpTrunc
                }
            }
            _ => return val,
        };
        let dst = self.fresh_value();
        self.emit(Some(dst), InstrKind::Cast { kind, from, to, operand: val });
        Operand::Val(dst)
    }

    fn zero_of(&self, ty: &Type) -> Operand {
        match ty {
            Type::Float(_) => Operand::ConstFloat(0.0, ty.clone()),
            Type::Bool => Operand::ConstBool(false),
            _ => Operand::ConstInt(0, ty.clone()),
        }
    }

    fn load_slot(&mut self, slot: SlotId, _ty: Type) -> ValueId {
        let dst = self.fresh_value();
        self.emit(Some(dst), InstrKind::Load { slot });
        dst
    }

    fn unsupported(&mut self, span: lime_span::Span, msg: &str) {
        self.diags.emit(
            Diagnostic::error(codes::E_UNSUPPORTED, msg.to_string())
                .primary(span, "fitur ini belum diimplementasikan pada backend"),
        );
    }

    fn fresh_value(&mut self) -> ValueId {
        let v = ValueId(self.next_value);
        self.next_value += 1;
        v
    }

    fn new_slot(&mut self, ty: Type) -> SlotId {
        let id = SlotId(self.slots.len() as u32);
        self.slots.push(Slot { id, ty });
        id
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(Block { id, instrs: Vec::new(), term: None });
        id
    }

    fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.0 as usize]
    }

    fn emit(&mut self, dst: Option<ValueId>, kind: InstrKind) {
        let cur = self.current.0 as usize;
        if self.blocks[cur].term.is_some() {
            return;
        }
        self.blocks[cur].instrs.push(Instr { dst, kind });
    }

    fn set_term(&mut self, term: Terminator) {
        let cur = self.current.0 as usize;
        if self.blocks[cur].term.is_none() {
            self.blocks[cur].term = Some(term);
        }
    }
}

