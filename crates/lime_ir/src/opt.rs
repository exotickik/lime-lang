use crate::ir::*;
use lime_types::Type;
use std::collections::{HashMap, HashSet};

pub fn optimize(module: &mut Module) {
    for func in &mut module.funcs {
        loop {
            let folded = fold_constants(func);
            let removed = eliminate_dead(func);
            if !folded && !removed {
                break;
            }
        }
    }
}

fn int_width_bits(ty: &Type) -> u32 {
    match ty {
        Type::Int { width, .. } => width.bits(),
        Type::Char => 32,
        _ => 64,
    }
}

fn mask(value: u128, bits: u32) -> u128 {
    if bits >= 128 {
        value
    } else {
        value & ((1u128 << bits) - 1)
    }
}

fn as_signed(value: u128, bits: u32) -> i128 {
    let m = mask(value, bits);
    if bits < 128 && (m >> (bits - 1)) & 1 == 1 {
        (m as i128) - (1i128 << bits)
    } else {
        m as i128
    }
}

fn fold_constants(func: &mut Func) -> bool {
    let mut known: HashMap<u32, Operand> = HashMap::new();
    let mut changed = false;

    loop {
        let mut new_known = false;
        for block in &func.blocks {
            for instr in &block.instrs {
                let Some(dst) = instr.dst else { continue };
                if known.contains_key(&dst.0) {
                    continue;
                }
                if let Some(c) = try_fold(&instr.kind, &known) {
                    known.insert(dst.0, c);
                    new_known = true;
                }
            }
        }
        if !new_known {
            break;
        }
    }

    if known.is_empty() {
        return false;
    }

    for block in &mut func.blocks {
        for instr in &mut block.instrs {
            changed |= subst_kind(&mut instr.kind, &known);
        }
        if let Some(term) = &mut block.term {
            changed |= subst_term(term, &known);
        }
    }
    changed
}

fn resolve(op: &Operand, known: &HashMap<u32, Operand>) -> Operand {
    match op {
        Operand::Val(v) => known.get(&v.0).cloned().unwrap_or_else(|| op.clone()),
        other => other.clone(),
    }
}

fn try_fold(kind: &InstrKind, known: &HashMap<u32, Operand>) -> Option<Operand> {
    match kind {
        InstrKind::Bin { op, ty, .. } if ty.is_integer() => {
            let (l, r) = bin_ints(kind, known)?;
            let bits = int_width_bits(ty);
            let res = match op {
                ArithOp::Add => l.wrapping_add(r),
                ArithOp::Sub => l.wrapping_sub(r),
                ArithOp::Mul => l.wrapping_mul(r),
                _ => return None,
            };
            Some(Operand::ConstInt(mask(res, bits), ty.clone()))
        }
        InstrKind::Cmp { pred, ty, signed, .. } if ty.is_integer() => {
            let (l, r) = cmp_ints(kind, known)?;
            let bits = int_width_bits(ty);
            let res = if *signed {
                let ls = as_signed(l, bits);
                let rs = as_signed(r, bits);
                eval_cmp(*pred, ls, rs)
            } else {
                eval_cmp(*pred, mask(l, bits), mask(r, bits))
            };
            Some(Operand::ConstBool(res))
        }
        InstrKind::Neg { ty, operand } if ty.is_integer() => {
            if let Operand::ConstInt(v, _) = resolve(operand, known) {
                let bits = int_width_bits(ty);
                Some(Operand::ConstInt(mask(0u128.wrapping_sub(v), bits), ty.clone()))
            } else {
                None
            }
        }
        InstrKind::Not { operand } => {
            if let Operand::ConstBool(b) = resolve(operand, known) {
                Some(Operand::ConstBool(!b))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn bin_ints(kind: &InstrKind, known: &HashMap<u32, Operand>) -> Option<(u128, u128)> {
    if let InstrKind::Bin { lhs, rhs, .. } = kind {
        if let (Operand::ConstInt(a, _), Operand::ConstInt(b, _)) =
            (resolve(lhs, known), resolve(rhs, known))
        {
            return Some((a, b));
        }
    }
    None
}

fn cmp_ints(kind: &InstrKind, known: &HashMap<u32, Operand>) -> Option<(u128, u128)> {
    if let InstrKind::Cmp { lhs, rhs, .. } = kind {
        if let (Operand::ConstInt(a, _), Operand::ConstInt(b, _)) =
            (resolve(lhs, known), resolve(rhs, known))
        {
            return Some((a, b));
        }
    }
    None
}

fn eval_cmp<T: PartialOrd + PartialEq>(pred: CmpPred, l: T, r: T) -> bool {
    match pred {
        CmpPred::Eq => l == r,
        CmpPred::Ne => l != r,
        CmpPred::Lt => l < r,
        CmpPred::Le => l <= r,
        CmpPred::Gt => l > r,
        CmpPred::Ge => l >= r,
    }
}

fn subst_operand(op: &mut Operand, known: &HashMap<u32, Operand>) -> bool {
    if let Operand::Val(v) = op {
        if let Some(c) = known.get(&v.0) {
            *op = c.clone();
            return true;
        }
    }
    false
}

fn subst_kind(kind: &mut InstrKind, known: &HashMap<u32, Operand>) -> bool {
    let mut changed = false;
    match kind {
        InstrKind::Store { value, .. } => changed |= subst_operand(value, known),
        InstrKind::Bin { lhs, rhs, .. } | InstrKind::Cmp { lhs, rhs, .. } => {
            changed |= subst_operand(lhs, known);
            changed |= subst_operand(rhs, known);
        }
        InstrKind::Neg { operand, .. }
        | InstrKind::Not { operand }
        | InstrKind::Cast { operand, .. } => changed |= subst_operand(operand, known),
        InstrKind::Select { cond, then_val, else_val, .. } => {
            changed |= subst_operand(cond, known);
            changed |= subst_operand(then_val, known);
            changed |= subst_operand(else_val, known);
        }
        InstrKind::Call { args, .. } => {
            for a in args {
                changed |= subst_operand(a, known);
            }
        }
        InstrKind::MakeStruct { fields, .. } => {
            for f in fields {
                changed |= subst_operand(f, known);
            }
        }
        InstrKind::ExtractField { agg, .. } => changed |= subst_operand(agg, known),
        InstrKind::InsertField { agg, value, .. } => {
            changed |= subst_operand(agg, known);
            changed |= subst_operand(value, known);
        }
        InstrKind::MakeArray { elems, .. } => {
            for e in elems {
                changed |= subst_operand(e, known);
            }
        }
        InstrKind::Gep { indices, .. } => {
            for idx in indices {
                if let GepIndex::ArrayVal(op) = idx {
                    changed |= subst_operand(op, known);
                }
            }
        }
        InstrKind::GepPtr { base, indices, .. } => {
            changed |= subst_operand(base, known);
            for idx in indices {
                if let GepIndex::ArrayVal(op) = idx {
                    changed |= subst_operand(op, known);
                }
            }
        }
        InstrKind::LoadPtr { ptr, .. } => changed |= subst_operand(ptr, known),
        InstrKind::StorePtr { ptr, value, .. } => {
            changed |= subst_operand(ptr, known);
            changed |= subst_operand(value, known);
        }
        InstrKind::KotakNew { value, .. } => changed |= subst_operand(value, known),
        InstrKind::Free { ptr } => changed |= subst_operand(ptr, known),
        InstrKind::Load { .. } | InstrKind::AddrOf { .. } => {}
    }
    changed
}

fn subst_term(term: &mut Terminator, known: &HashMap<u32, Operand>) -> bool {
    match term {
        Terminator::Ret(Some(op)) => subst_operand(op, known),
        Terminator::CondBr { cond, .. } => subst_operand(cond, known),
        _ => false,
    }
}

fn eliminate_dead(func: &mut Func) -> bool {
    let mut used: HashSet<u32> = HashSet::new();
    for block in &func.blocks {
        for instr in &block.instrs {
            collect_uses(&instr.kind, &mut used);
        }
        if let Some(term) = &block.term {
            collect_term_uses(term, &mut used);
        }
    }

    let mut changed = false;
    for block in &mut func.blocks {
        let before = block.instrs.len();
        block.instrs.retain(|instr| match instr.dst {
            Some(d) if is_pure(&instr.kind) => used.contains(&d.0),
            _ => true,
        });
        if block.instrs.len() != before {
            changed = true;
        }
    }
    changed
}

fn is_pure(kind: &InstrKind) -> bool {
    matches!(
        kind,
        InstrKind::Load { .. }
            | InstrKind::Bin { .. }
            | InstrKind::Cmp { .. }
            | InstrKind::Neg { .. }
            | InstrKind::Not { .. }
            | InstrKind::Cast { .. }
            | InstrKind::Select { .. }
            | InstrKind::MakeStruct { .. }
            | InstrKind::ExtractField { .. }
            | InstrKind::InsertField { .. }
            | InstrKind::MakeArray { .. }
            | InstrKind::Gep { .. }
            | InstrKind::GepPtr { .. }
            | InstrKind::LoadPtr { .. }
            | InstrKind::AddrOf { .. }
    )
}

fn note_use(op: &Operand, used: &mut HashSet<u32>) {
    if let Operand::Val(v) = op {
        used.insert(v.0);
    }
}

fn collect_uses(kind: &InstrKind, used: &mut HashSet<u32>) {
    match kind {
        InstrKind::Store { value, .. } => note_use(value, used),
        InstrKind::Bin { lhs, rhs, .. } | InstrKind::Cmp { lhs, rhs, .. } => {
            note_use(lhs, used);
            note_use(rhs, used);
        }
        InstrKind::Neg { operand, .. }
        | InstrKind::Not { operand }
        | InstrKind::Cast { operand, .. } => note_use(operand, used),
        InstrKind::Select { cond, then_val, else_val, .. } => {
            note_use(cond, used);
            note_use(then_val, used);
            note_use(else_val, used);
        }
        InstrKind::Call { args, .. } => {
            for a in args {
                note_use(a, used);
            }
        }
        InstrKind::MakeStruct { fields, .. } => {
            for f in fields {
                note_use(f, used);
            }
        }
        InstrKind::ExtractField { agg, .. } => note_use(agg, used),
        InstrKind::InsertField { agg, value, .. } => {
            note_use(agg, used);
            note_use(value, used);
        }
        InstrKind::MakeArray { elems, .. } => {
            for e in elems {
                note_use(e, used);
            }
        }
        InstrKind::Gep { indices, .. } => {
            for idx in indices {
                if let GepIndex::ArrayVal(op) = idx {
                    note_use(op, used);
                }
            }
        }
        InstrKind::GepPtr { base, indices, .. } => {
            note_use(base, used);
            for idx in indices {
                if let GepIndex::ArrayVal(op) = idx {
                    note_use(op, used);
                }
            }
        }
        InstrKind::LoadPtr { ptr, .. } => note_use(ptr, used),
        InstrKind::StorePtr { ptr, value, .. } => {
            note_use(ptr, used);
            note_use(value, used);
        }
        InstrKind::KotakNew { value, .. } => note_use(value, used),
        InstrKind::Free { ptr } => note_use(ptr, used),
        InstrKind::Load { .. } | InstrKind::AddrOf { .. } => {}
    }
}

fn collect_term_uses(term: &Terminator, used: &mut HashSet<u32>) {
    match term {
        Terminator::Ret(Some(op)) => note_use(op, used),
        Terminator::CondBr { cond, .. } => note_use(cond, used),
        _ => {}
    }
}
