use lime_types::Type;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SlotId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GlobalId(pub u32);

#[derive(Clone, Debug)]
pub enum Operand {
    Val(ValueId),
    ConstInt(u128, Type),
    ConstFloat(f64, Type),
    ConstBool(bool),
    Str(GlobalId),
    Unit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmpPred {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CastKind {
    SExt,
    ZExt,
    Trunc,
    FpExt,
    FpTrunc,
}

#[derive(Clone, Debug)]
pub enum InstrKind {
    Load { slot: SlotId },
    Store { slot: SlotId, value: Operand },
    Bin { op: ArithOp, ty: Type, signed: bool, lhs: Operand, rhs: Operand },
    Cmp { pred: CmpPred, ty: Type, signed: bool, lhs: Operand, rhs: Operand },
    Neg { ty: Type, operand: Operand },
    Not { operand: Operand },
    Cast { kind: CastKind, from: Type, to: Type, operand: Operand },
    Select { cond: Operand, then_val: Operand, else_val: Operand, ty: Type },
    Call { callee: String, args: Vec<Operand>, ret: Type, variadic_from: Option<usize> },
    MakeStruct { ty: Type, fields: Vec<Operand> },
    ExtractField { agg: Operand, agg_ty: Type, index: u32, field_ty: Type },
    InsertField { agg: Operand, agg_ty: Type, indices: Vec<u32>, value: Operand, field_ty: Type },
    MakeArray { elem_ty: Type, elems: Vec<Operand> },
    Gep { slot: SlotId, agg_ty: Type, indices: Vec<GepIndex> },
    GepPtr { base: Operand, agg_ty: Type, indices: Vec<GepIndex> },
    LoadPtr { ptr: Operand, ty: Type },
    StorePtr { ptr: Operand, ty: Type, value: Operand },
    KotakNew { inner_ty: Type, value: Operand },
    Free { ptr: Operand },
    AddrOf { slot: SlotId, ty: Type },
}

#[derive(Clone, Debug)]
pub enum GepIndex {
    Field(u32),
    ArrayConst(u64),
    ArrayVal(Operand),
}

#[derive(Clone, Debug)]
pub struct Instr {
    pub dst: Option<ValueId>,
    pub kind: InstrKind,
}

#[derive(Clone, Debug)]
pub enum Terminator {
    Ret(Option<Operand>),
    Br(BlockId),
    CondBr { cond: Operand, then_bb: BlockId, else_bb: BlockId },
    Unreachable,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: BlockId,
    pub instrs: Vec<Instr>,
    pub term: Option<Terminator>,
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub id: SlotId,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub params: Vec<(ValueId, Type)>,
    pub ret: Type,
    pub slots: Vec<Slot>,
    pub blocks: Vec<Block>,
    pub entry: BlockId,
    pub is_main: bool,
}

#[derive(Clone, Debug)]
pub struct ExternSig {
    pub name: String,
    pub params: Vec<Type>,
    pub variadic: bool,
    pub ret: Type,
}

#[derive(Clone, Debug)]
pub struct GlobalStr {
    pub id: GlobalId,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct IrStruct {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct IrEnum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Module {
    pub name: String,
    pub structs: Vec<IrStruct>,
    pub enums: Vec<IrEnum>,
    pub externs: Vec<ExternSig>,
    pub globals: Vec<GlobalStr>,
    pub funcs: Vec<Func>,
    pub needs_printf: bool,
    pub needs_abort: bool,
    pub needs_strcmp: bool,
    pub needs_strlen: bool,
    pub needs_malloc: bool,
    pub needs_free: bool,
}

impl Module {
    pub fn intern_str(&mut self, bytes: Vec<u8>) -> GlobalId {
        for g in &self.globals {
            if g.bytes == bytes {
                return g.id;
            }
        }
        let id = GlobalId(self.globals.len() as u32);
        self.globals.push(GlobalStr { id, bytes });
        id
    }
}
