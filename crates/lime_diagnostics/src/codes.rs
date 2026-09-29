//! Stable diagnostic codes.
//!
//! Codes are stable identifiers referenced by tests and (eventually) user
//! documentation. Grouping:
//!   E01xx  lexer / parser
//!   E02xx  name resolution / type checking / control flow
//!   E03xx  assignment / l-values
//!   E09xx  backend / toolchain
//!   W0xxx  warnings

/// A diagnostic code such as `E0201`. Cheap to copy and compare.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Code(pub &'static str);

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// --- Lexer / parser -------------------------------------------------------
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

// --- Name resolution / types ---------------------------------------------
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

// --- Assignment / l-values ------------------------------------------------
pub const E_INVALID_ASSIGN_TARGET: Code = Code("E0301");

// --- Backend / toolchain --------------------------------------------------
pub const E_TOOLCHAIN: Code = Code("E0901");
pub const E_UNSUPPORTED: Code = Code("E0902");

// --- Warnings -------------------------------------------------------------
pub const W_UNREACHABLE: Code = Code("W0010");
pub const W_UNUSED_BINDING: Code = Code("W0011");
