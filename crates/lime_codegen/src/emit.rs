use crate::target::Target;
use lime_ir::*;
use lime_types::Type;
use std::collections::HashMap;
use std::fmt::Write;

pub fn emit_module(module: &Module, target: &Target) -> String {
    let mut out = String::new();
    writeln!(out, "; Lime IR -> LLVM IR for program '{}'", module.name).ok();
    writeln!(out, "target triple = \"{}\"", target.triple).ok();
    out.push('\n');

    for (i, s) in module.structs.iter().enumerate() {
        let ftys: Vec<String> = s.fields.iter().map(ty_str).collect();
        writeln!(out, "%struct.{} = type {{ {} }}", i, ftys.join(", ")).ok();
    }
    if !module.structs.is_empty() {
        out.push('\n');
    }

    for g in &module.globals {
        emit_global(&mut out, g);
    }
    if !module.globals.is_empty() {
        out.push('\n');
    }

    if module.needs_printf {
        writeln!(out, "declare i32 @printf(ptr, ...)").ok();
    }
    for e in &module.externs {
        emit_extern_decl(&mut out, e);
    }
    if module.needs_printf || !module.externs.is_empty() {
        out.push('\n');
    }

    for func in &module.funcs {
        emit_func(&mut out, func, &module.structs);
        out.push('\n');
    }

    out
}

fn emit_global(out: &mut String, g: &GlobalStr) {
    let len = g.bytes.len() + 1;
    let mut content = String::new();
    for &b in &g.bytes {
        if b == b'"' || b == b'\\' || b < 0x20 || b > 0x7e {
            content.push_str(&format!("\\{:02X}", b));
        } else {
            content.push(b as char);
        }
    }
    content.push_str("\\00");
    writeln!(
        out,
        "@.str{} = private unnamed_addr constant [{} x i8] c\"{}\"",
        g.id.0, len, content
    )
    .ok();
}

fn emit_extern_decl(out: &mut String, e: &ExternSig) {
    let ret = ty_str(&e.ret);
    let mut params: Vec<String> = e.params.iter().map(ty_str).collect();
    if e.variadic {
        params.push("...".to_string());
    }
    writeln!(out, "declare {} @{}({})", ret, e.name, params.join(", ")).ok();
}

fn emit_func(out: &mut String, func: &Func, structs: &[IrStruct]) {
    let value_types = collect_value_types(func);
    let ret = ty_str(&func.ret);
    let params: Vec<String> =
        func.params.iter().map(|(v, t)| format!("{} %v{}", ty_str(t), v.0)).collect();
    writeln!(out, "define {} @{}({}) {{", ret, func.name, params.join(", ")).ok();

    for (i, block) in func.blocks.iter().enumerate() {
        writeln!(out, "bb{}:", block.id.0).ok();
        if i == 0 {
            for slot in &func.slots {
                writeln!(out, "  %slot{} = alloca {}", slot.id.0, ty_str(&slot.ty)).ok();
            }
        }
        for instr in &block.instrs {
            emit_instr(out, instr, func, &value_types, structs);
        }
        match &block.term {
            Some(t) => emit_term(out, t, &value_types),
            None => {
                writeln!(out, "  unreachable").ok();
            }
        }
    }

    writeln!(out, "}}").ok();
}

fn slot_ty<'a>(func: &'a Func, slot: SlotId) -> &'a Type {
    &func.slots[slot.0 as usize].ty
}

fn emit_instr(
    out: &mut String,
    instr: &Instr,
    func: &Func,
    vt: &HashMap<u32, Type>,
    structs: &[IrStruct],
) {
    let dst = instr.dst.map(|d| format!("%v{}", d.0));
    match &instr.kind {
        InstrKind::Load { slot } => {
            let t = ty_str(slot_ty(func, *slot));
            writeln!(out, "  {} = load {}, ptr %slot{}", dst.unwrap(), t, slot.0).ok();
        }
        InstrKind::Store { slot, value } => {
            let t = ty_str(slot_ty(func, *slot));
            writeln!(out, "  store {} {}, ptr %slot{}", t, val_str(value, vt), slot.0).ok();
        }
        InstrKind::Bin { op, ty, signed, lhs, rhs } => {
            let t = ty_str(ty);
            let opname = arith_opname(*op, ty, *signed);
            writeln!(
                out,
                "  {} = {} {} {}, {}",
                dst.unwrap(),
                opname,
                t,
                val_str(lhs, vt),
                val_str(rhs, vt)
            )
            .ok();
        }
        InstrKind::Cmp { pred, ty, signed, lhs, rhs } => {
            let t = ty_str(ty);
            let (inst, p) = cmp_opname(*pred, ty, *signed);
            writeln!(
                out,
                "  {} = {} {} {} {}, {}",
                dst.unwrap(),
                inst,
                p,
                t,
                val_str(lhs, vt),
                val_str(rhs, vt)
            )
            .ok();
        }
        InstrKind::Neg { ty, operand } => {
            let t = ty_str(ty);
            if ty.is_float() {
                writeln!(out, "  {} = fneg {} {}", dst.unwrap(), t, val_str(operand, vt)).ok();
            } else {
                writeln!(out, "  {} = sub {} 0, {}", dst.unwrap(), t, val_str(operand, vt)).ok();
            }
        }
        InstrKind::Not { operand } => {
            writeln!(out, "  {} = xor i1 {}, true", dst.unwrap(), val_str(operand, vt)).ok();
        }
        InstrKind::Cast { kind, from, to, operand } => {
            let op = cast_opname(*kind);
            writeln!(
                out,
                "  {} = {} {} {} to {}",
                dst.unwrap(),
                op,
                ty_str(from),
                val_str(operand, vt),
                ty_str(to)
            )
            .ok();
        }
        InstrKind::Select { cond, then_val, else_val, ty } => {
            let t = ty_str(ty);
            writeln!(
                out,
                "  {} = select i1 {}, {} {}, {} {}",
                dst.unwrap(),
                val_str(cond, vt),
                t,
                val_str(then_val, vt),
                t,
                val_str(else_val, vt)
            )
            .ok();
        }
        InstrKind::Call { callee, args, ret, variadic_from } => {
            let arg_strs: Vec<String> =
                args.iter().map(|a| format!("{} {}", operand_ty_str(a, vt), val_str(a, vt))).collect();
            let ret_s = ty_str(ret);
            let assign = match &dst {
                Some(d) => format!("{} = ", d),
                None => String::new(),
            };
            match variadic_from {
                Some(fixed) => {
                    let fixed_types: Vec<String> =
                        args[..*fixed].iter().map(|a| operand_ty_str(a, vt)).collect();
                    let sig = format!("{}, ...", fixed_types.join(", "));
                    writeln!(out, "  {}call {} ({}) @{}({})", assign, ret_s, sig, callee, arg_strs.join(", "))
                        .ok();
                }
                None => {
                    writeln!(out, "  {}call {} @{}({})", assign, ret_s, callee, arg_strs.join(", "))
                        .ok();
                }
            }
        }
        InstrKind::MakeStruct { ty, fields } => {
            let struct_ty = ty_str(ty);
            let field_types: &[Type] = match ty {
                Type::Struct(id) => &structs[id.0 as usize].fields,
                _ => &[],
            };
            let d = dst.unwrap();
            let n = fields.len();
            if n == 0 {
                writeln!(out, "  {} = freeze {} zeroinitializer", d, struct_ty).ok();
            } else {
                let mut cur = "undef".to_string();
                for (i, f) in fields.iter().enumerate() {
                    let fty = field_types.get(i).cloned().unwrap_or(Type::I64);
                    let name = if i + 1 == n { d.clone() } else { format!("{}.f{}", d, i) };
                    writeln!(
                        out,
                        "  {} = insertvalue {} {}, {} {}, {}",
                        name,
                        struct_ty,
                        cur,
                        ty_str(&fty),
                        val_str(f, vt),
                        i
                    )
                    .ok();
                    cur = name;
                }
            }
        }
        InstrKind::ExtractField { agg, agg_ty, index, .. } => {
            writeln!(
                out,
                "  {} = extractvalue {} {}, {}",
                dst.unwrap(),
                ty_str(agg_ty),
                val_str(agg, vt),
                index
            )
            .ok();
        }
        InstrKind::InsertField { agg, agg_ty, indices, value, field_ty } => {
            let idxs: Vec<String> = indices.iter().map(|i| i.to_string()).collect();
            writeln!(
                out,
                "  {} = insertvalue {} {}, {} {}, {}",
                dst.unwrap(),
                ty_str(agg_ty),
                val_str(agg, vt),
                ty_str(field_ty),
                val_str(value, vt),
                idxs.join(", ")
            )
            .ok();
        }
    }
}

fn emit_term(out: &mut String, term: &Terminator, vt: &HashMap<u32, Type>) {
    match term {
        Terminator::Ret(Some(op)) => {
            writeln!(out, "  ret {} {}", operand_ty_str(op, vt), val_str(op, vt)).ok();
        }
        Terminator::Ret(None) => {
            writeln!(out, "  ret void").ok();
        }
        Terminator::Br(bb) => {
            writeln!(out, "  br label %bb{}", bb.0).ok();
        }
        Terminator::CondBr { cond, then_bb, else_bb } => {
            writeln!(
                out,
                "  br i1 {}, label %bb{}, label %bb{}",
                val_str(cond, vt),
                then_bb.0,
                else_bb.0
            )
            .ok();
        }
        Terminator::Unreachable => {
            writeln!(out, "  unreachable").ok();
        }
    }
}

fn collect_value_types(func: &Func) -> HashMap<u32, Type> {
    let mut m = HashMap::new();
    for (v, t) in &func.params {
        m.insert(v.0, t.clone());
    }
    for block in &func.blocks {
        for instr in &block.instrs {
            let Some(dst) = instr.dst else { continue };
            let ty = match &instr.kind {
                InstrKind::Load { slot } => func.slots[slot.0 as usize].ty.clone(),
                InstrKind::Bin { ty, .. } => ty.clone(),
                InstrKind::Cmp { .. } => Type::Bool,
                InstrKind::Neg { ty, .. } => ty.clone(),
                InstrKind::Not { .. } => Type::Bool,
                InstrKind::Cast { to, .. } => to.clone(),
                InstrKind::Select { ty, .. } => ty.clone(),
                InstrKind::Call { ret, .. } => ret.clone(),
                InstrKind::MakeStruct { ty, .. } => ty.clone(),
                InstrKind::ExtractField { field_ty, .. } => field_ty.clone(),
                InstrKind::InsertField { agg_ty, .. } => agg_ty.clone(),
                InstrKind::Store { .. } => continue,
            };
            m.insert(dst.0, ty);
        }
    }
    m
}

fn ty_str(ty: &Type) -> String {
    match ty {
        Type::Int { width, .. } => format!("i{}", width.bits()),
        Type::Float(w) => {
            if w.bits() == 32 {
                "float".to_string()
            } else {
                "double".to_string()
            }
        }
        Type::Bool => "i1".to_string(),
        Type::Char => "i32".to_string(),
        Type::Str => "ptr".to_string(),
        Type::Unit => "void".to_string(),
        Type::Struct(id) => format!("%struct.{}", id.0),
        Type::Enum(_) => "i32".to_string(),
        Type::Fn { .. } => "ptr".to_string(),
        Type::Error => "void".to_string(),
    }
}

fn operand_ty_str(op: &Operand, vt: &HashMap<u32, Type>) -> String {
    match op {
        Operand::Val(v) => ty_str(vt.get(&v.0).unwrap_or(&Type::I64)),
        Operand::ConstInt(_, ty) => ty_str(ty),
        Operand::ConstFloat(_, ty) => ty_str(ty),
        Operand::ConstBool(_) => "i1".to_string(),
        Operand::Str(_) => "ptr".to_string(),
        Operand::Unit => "void".to_string(),
    }
}

fn val_str(op: &Operand, _vt: &HashMap<u32, Type>) -> String {
    match op {
        Operand::Val(v) => format!("%v{}", v.0),
        Operand::ConstInt(v, ty) => const_int_str(*v, ty),
        Operand::ConstFloat(v, ty) => const_float_str(*v, ty),
        Operand::ConstBool(b) => if *b { "true".to_string() } else { "false".to_string() },
        Operand::Str(id) => format!("@.str{}", id.0),
        Operand::Unit => "void".to_string(),
    }
}

fn const_int_str(v: u128, ty: &Type) -> String {
    match ty {
        Type::Int { width, signed } => {
            let bits = width.bits();
            if *signed {
                format!("{}", as_signed(v, bits))
            } else {
                format!("{}", mask(v, bits))
            }
        }
        Type::Char => format!("{}", mask(v, 32)),
        Type::Enum(_) => format!("{}", mask(v, 32)),
        Type::Bool => if v != 0 { "true".to_string() } else { "false".to_string() },
        _ => format!("{}", v),
    }
}

fn const_float_str(v: f64, ty: &Type) -> String {
    let bits = if *ty == Type::F32 {
        (v as f32 as f64).to_bits()
    } else {
        v.to_bits()
    };
    format!("0x{:016X}", bits)
}

fn arith_opname(op: ArithOp, ty: &Type, signed: bool) -> &'static str {
    if ty.is_float() {
        match op {
            ArithOp::Add => "fadd",
            ArithOp::Sub => "fsub",
            ArithOp::Mul => "fmul",
            ArithOp::Div => "fdiv",
            ArithOp::Rem => "frem",
        }
    } else {
        match op {
            ArithOp::Add => "add",
            ArithOp::Sub => "sub",
            ArithOp::Mul => "mul",
            ArithOp::Div => if signed { "sdiv" } else { "udiv" },
            ArithOp::Rem => if signed { "srem" } else { "urem" },
        }
    }
}

fn cmp_opname(pred: CmpPred, ty: &Type, signed: bool) -> (&'static str, &'static str) {
    if ty.is_float() {
        let p = match pred {
            CmpPred::Eq => "oeq",
            CmpPred::Ne => "one",
            CmpPred::Lt => "olt",
            CmpPred::Le => "ole",
            CmpPred::Gt => "ogt",
            CmpPred::Ge => "oge",
        };
        ("fcmp", p)
    } else {
        let p = match (pred, signed) {
            (CmpPred::Eq, _) => "eq",
            (CmpPred::Ne, _) => "ne",
            (CmpPred::Lt, true) => "slt",
            (CmpPred::Le, true) => "sle",
            (CmpPred::Gt, true) => "sgt",
            (CmpPred::Ge, true) => "sge",
            (CmpPred::Lt, false) => "ult",
            (CmpPred::Le, false) => "ule",
            (CmpPred::Gt, false) => "ugt",
            (CmpPred::Ge, false) => "uge",
        };
        ("icmp", p)
    }
}

fn cast_opname(kind: CastKind) -> &'static str {
    match kind {
        CastKind::SExt => "sext",
        CastKind::ZExt => "zext",
        CastKind::Trunc => "trunc",
        CastKind::FpExt => "fpext",
        CastKind::FpTrunc => "fptrunc",
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
