pub mod check;
pub mod results;

pub use check::Checker;
pub use results::{
    Builtin, FieldInfo, FnSig, LocalId, LocalInfo, Res, StructInfo, TypeckResults,
};

use lime_ast::Program;
use lime_diagnostics::DiagnosticEngine;

pub fn analyze(program: &Program, diags: &mut DiagnosticEngine) -> TypeckResults {
    Checker::new(diags).analyze(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lime_diagnostics::{codes, Code, DiagnosticEngine};
    use lime_lexer::tokenize;
    use lime_parser::parse;
    use lime_span::SourceMap;

    fn check(src: &str) -> DiagnosticEngine {
        let mut sm = SourceMap::new();
        let f = sm.add("t.lime", src);
        let mut diags = DiagnosticEngine::new();
        let toks = tokenize(f, src, &mut diags);
        let prog = parse(f, toks, &mut diags);
        let _ = analyze(&prog, &mut diags);
        diags
    }

    fn has_code(d: &DiagnosticEngine, code: Code) -> bool {
        d.diagnostics().iter().any(|x| x.code == Some(code))
    }

    #[test]
    fn valid_program_has_no_errors() {
        let d = check(
            "program P\n\
             fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
             utama { nilai x = tambah(2, 3) tampilkan(x) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn undefined_name() {
        let d = check("program P\nutama { tampilkan(umur) }");
        assert!(has_code(&d, codes::E_UNDEFINED_NAME));
    }

    #[test]
    fn type_mismatch_binop() {
        let d = check(
            "program P\nutama { nilai a: i32 = 1i32 nilai b: i64 = 2i64 tampilkan(a + b) }",
        );
        assert!(has_code(&d, codes::E_TYPE_MISMATCH_BINOP));
    }

    #[test]
    fn assign_to_immutable() {
        let d = check("program P\nutama { nilai x = 1 x = 2 }");
        assert!(has_code(&d, codes::E_ASSIGN_IMMUTABLE));
    }

    #[test]
    fn mutable_assign_ok() {
        let d = check("program P\nutama { ubah x = 1 x = 2 tampilkan(x) }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn missing_return_detected() {
        let d = check(
            "program P\nfungsi f(x: angka) -> angka { jika x > 0 { kembalikan 1 } }\nutama { }",
        );
        assert!(has_code(&d, codes::E_MISSING_RETURN));
    }

    #[test]
    fn all_paths_return_ok() {
        let d = check(
            "program P\n\
             fungsi f(x: angka) -> angka { jika x > 0 { kembalikan 1 } lain { kembalikan 2 } }\n\
             utama { tampilkan(f(1)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn arity_mismatch() {
        let d = check(
            "program P\nfungsi g(a: angka) -> angka { kembalikan a }\nutama { tampilkan(g(1, 2)) }",
        );
        assert!(has_code(&d, codes::E_ARITY_MISMATCH));
    }

    #[test]
    fn break_outside_loop() {
        let d = check("program P\nutama { berhenti }");
        assert!(has_code(&d, codes::E_LOOP_CTRL_OUTSIDE_LOOP));
    }

    #[test]
    fn condition_must_be_bool() {
        let d = check("program P\nutama { jika 1 { } }");
        assert!(has_code(&d, codes::E_INVALID_OPERAND));
    }

    #[test]
    fn missing_main_reported() {
        let d = check("program P\nfungsi f() { }");
        assert!(has_code(&d, codes::E_MISSING_MAIN));
    }

    #[test]
    fn for_loop_declares_var() {
        let d = check("program P\nutama { untuk i dari 0 sampai 10 { tampilkan(i) } }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn unknown_type_reported() {
        let d = check("program P\nfungsi f(a: bilangan) { }\nutama { }");
        assert!(has_code(&d, codes::E_UNKNOWN_TYPE));
    }

    #[test]
    fn valid_struct_literal() {
        let d = check(
            "program P\ntipe Titik { x: i32\n y: i32 }\n\
             utama { nilai t = Titik { x: 1i32, y: 2i32 } tampilkan(t.x) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn struct_literal_missing_field() {
        let d = check(
            "program P\ntipe Titik { x: i32\n y: i32 }\nutama { nilai t = Titik { x: 1i32 } }",
        );
        assert!(has_code(&d, codes::E_MISSING_FIELD));
    }

    #[test]
    fn struct_literal_unknown_field() {
        let d = check(
            "program P\ntipe Titik { x: i32 }\nutama { nilai t = Titik { x: 1i32, z: 2i32 } }",
        );
        assert!(has_code(&d, codes::E_UNKNOWN_FIELD));
    }

    #[test]
    fn struct_literal_field_type_mismatch() {
        let d = check(
            "program P\ntipe Titik { x: i32 }\nutama { nilai t = Titik { x: benar } }",
        );
        assert!(has_code(&d, codes::E_TYPE_MISMATCH_ASSIGN));
    }

    #[test]
    fn struct_literal_duplicate_field() {
        let d = check(
            "program P\ntipe Titik { x: i32 }\nutama { nilai t = Titik { x: 1i32, x: 2i32 } }",
        );
        assert!(has_code(&d, codes::E_DUPLICATE_FIELD));
    }

    #[test]
    fn construct_non_struct_is_error() {
        let d = check("program P\nutama { nilai t = i32 { x: 1 } }");
        assert!(has_code(&d, codes::E_NOT_A_STRUCT));
    }

    #[test]
    fn unknown_field_access_reported() {
        let d = check(
            "program P\ntipe Titik { x: i32 }\n\
             utama { nilai t = Titik { x: 1i32 } tampilkan(t.z) }",
        );
        assert!(has_code(&d, codes::E_UNKNOWN_FIELD));
    }

    #[test]
    fn immutable_struct_field_assign_rejected() {
        let d = check(
            "program P\ntipe Titik { x: i32 }\n\
             utama { nilai t = Titik { x: 1i32 } t.x = 5i32 }",
        );
        assert!(has_code(&d, codes::E_ASSIGN_IMMUTABLE));
    }
}
