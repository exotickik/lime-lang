#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Code(pub &'static str);

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

pub const E_UNKNOWN_CHAR: Code = Code("E0101");
pub const E_UNTERMINATED_STRING: Code = Code("E0102");
pub const E_UNTERMINATED_COMMENT: Code = Code("E0103");
pub const E_BAD_ESCAPE: Code = Code("E0104");
pub const E_BAD_NUMBER: Code = Code("E0105");
pub const E_EXPECTED_TOKEN: Code = Code("E0110");
pub const E_UNEXPECTED_TOKEN: Code = Code("E0111");
pub const E_EXPECTED_ITEM: Code = Code("E0112");
pub const E_MISSING_PROGRAM_HEADER: Code = Code("E0113");
pub const E_DUPLICATE_MAIN: Code = Code("E0114");
pub const E_MISSING_MAIN: Code = Code("E0115");

pub const E_UNDEFINED_NAME: Code = Code("E0201");
pub const E_REDEFINITION: Code = Code("E0202");
pub const E_TYPE_MISMATCH_BINOP: Code = Code("E0210");
pub const E_INVALID_OPERAND: Code = Code("E0211");
pub const E_TYPE_MISMATCH_ASSIGN: Code = Code("E0212");
pub const E_UNKNOWN_TYPE: Code = Code("E0213");
pub const E_ARITY_MISMATCH: Code = Code("E0220");
pub const E_NOT_CALLABLE: Code = Code("E0221");
pub const E_MISSING_RETURN: Code = Code("E0230");
pub const E_RETURN_TYPE_MISMATCH: Code = Code("E0231");
pub const E_RETURN_OUTSIDE_FN: Code = Code("E0232");
pub const E_LOOP_CTRL_OUTSIDE_LOOP: Code = Code("E0240");
pub const E_ASSIGN_IMMUTABLE: Code = Code("E0250");
pub const E_UNKNOWN_FIELD: Code = Code("E0260");
pub const E_NOT_A_STRUCT: Code = Code("E0261");
pub const E_MISSING_FIELD: Code = Code("E0262");
pub const E_DUPLICATE_FIELD: Code = Code("E0263");
pub const E_UNKNOWN_VARIANT: Code = Code("E0265");
pub const E_NOT_INDEXABLE: Code = Code("E0266");
pub const E_NOT_ITERABLE: Code = Code("E0267");
pub const E_CANNOT_INFER: Code = Code("E0270");
pub const E_USE_AFTER_MOVE: Code = Code("E0280");
pub const E_INCONSISTENT_MOVE: Code = Code("E0281");
pub const E_MOVE_IN_LOOP: Code = Code("E0282");
pub const E_OWNED_IN_FFI: Code = Code("E0283");
pub const E_OWNED_REASSIGN: Code = Code("E0284");
pub const E_BORROW_NOT_PLACE: Code = Code("E0290");
pub const E_BORROW_IMMUTABLE: Code = Code("E0291");
pub const E_DEREF_NON_REF: Code = Code("E0292");
pub const E_DEREF_NON_COPY: Code = Code("E0293");
pub const E_STORE_THROUGH_SHARED: Code = Code("E0294");
pub const E_BORROW_CONFLICT: Code = Code("E0295");
pub const E_MUTATE_WHILE_BORROWED: Code = Code("E0296");
pub const E_REF_ESCAPE: Code = Code("E0297");
pub const E_REF_IN_FFI: Code = Code("E0298");

pub const E_TRAIT_DUP_METHOD: Code = Code("E02A0");
pub const E_IMPL_METHOD_MISMATCH: Code = Code("E02A1");
pub const E_METHOD_NOT_FOUND: Code = Code("E02A2");
pub const E_METHOD_AMBIGUOUS: Code = Code("E02A3");
pub const E_BOUND_NOT_SATISFIED: Code = Code("E02A4");
pub const E_DUP_IMPL: Code = Code("E02A5");
pub const E_UNKNOWN_TRAIT_OR_TYPE: Code = Code("E02A6");

pub const E_HASIL_NO_CONTEXT: Code = Code("E02B0");
pub const E_TRY_NON_HASIL: Code = Code("E02B1");
pub const E_TRY_BAD_CONTEXT: Code = Code("E02B2");
pub const E_HASIL_NOT_COPY: Code = Code("E02B3");
pub const E_INSPECT_MISUSE: Code = Code("E02B4");
pub const E_HASIL_TYPE_CONFLICT: Code = Code("E02B5");

pub const E_VISIBILITY: Code = Code("E02C0");

pub const E_THREAD_MISUSE: Code = Code("E02D0");

pub const E_COLLECTION_MISUSE: Code = Code("E02E0");
pub const E_INVALID_CAST: Code = Code("E02F0");

pub const E_INVALID_ASSIGN_TARGET: Code = Code("E0301");

pub const E_TOOLCHAIN: Code = Code("E0901");
pub const E_UNSUPPORTED: Code = Code("E0902");

pub const W_UNREACHABLE: Code = Code("W0010");
pub const W_UNUSED_BINDING: Code = Code("W0011");
