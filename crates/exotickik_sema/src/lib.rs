pub mod check;
pub mod ownership;
pub mod results;

pub use check::Checker;
pub use ownership::is_copy;
pub use results::{
    Builtin, EnumInfo, FieldInfo, FnSig, LocalId, LocalInfo, MethodResolution, RecvMode, Res,
    StructInfo, TypeckResults,
};

use exotickik_ast::Program;
use exotickik_diagnostics::DiagnosticEngine;
use exotickik_span::FileId;
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct ModuleInfo {
    pub module_names: Vec<String>,
    pub file_module: HashMap<FileId, usize>,
    pub imports: Vec<HashSet<String>>,
}

pub fn analyze(program: &Program, diags: &mut DiagnosticEngine) -> TypeckResults {
    Checker::new(diags).analyze(program)
}

pub fn analyze_modules(
    program: &Program,
    modules: &ModuleInfo,
    diags: &mut DiagnosticEngine,
) -> TypeckResults {
    let mut checker = Checker::new(diags);
    checker.set_modules(modules.clone());
    checker.analyze(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exotickik_diagnostics::{codes, Code, DiagnosticEngine};
    use exotickik_lexer::tokenize;
    use exotickik_parser::parse;
    use exotickik_span::SourceMap;

    fn check(src: &str) -> DiagnosticEngine {
        let mut sm = SourceMap::new();
        let f = sm.add("t.exo", src);
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

    #[test]
    fn valid_enum_usage() {
        let d = check(
            "program P\nenum Warna { Merah, Hijau, Biru }\n\
             utama { nilai w = Warna.Hijau tampilkan(w) jika w == Warna.Hijau { tampilkan(\"ya\") } }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn unknown_enum_variant() {
        let d = check(
            "program P\nenum Warna { Merah }\nutama { nilai w = Warna.Ungu }",
        );
        assert!(has_code(&d, codes::E_UNKNOWN_VARIANT));
    }

    #[test]
    fn enum_from_function_return() {
        let d = check(
            "program P\nenum Arah { Kiri, Kanan }\n\
             fungsi balik(a: Arah) -> Arah { kembalikan Arah.Kiri }\n\
             utama { tampilkan(balik(Arah.Kanan)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn enum_compared_to_int_is_error() {
        let d = check(
            "program P\nenum Arah { Kiri, Kanan }\nutama { jika Arah.Kiri == 0 { } }",
        );
        assert!(has_code(&d, codes::E_TYPE_MISMATCH_BINOP));
    }

    #[test]
    fn duplicate_enum_variant_rejected() {
        let d = check("program P\nenum E { A, A }\nutama { }");
        assert!(has_code(&d, codes::E_REDEFINITION));
    }

    #[test]
    fn valid_array_usage() {
        let d = check(
            "program P\nutama { nilai xs: [i32; 3] = [1i32, 2i32, 3i32] tampilkan(xs[0]) tampilkan(panjang(xs)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn array_element_type_mismatch() {
        let d = check("program P\nutama { nilai xs: [i32; 2] = [1i32, benar] }");
        assert!(has_code(&d, codes::E_TYPE_MISMATCH_ASSIGN));
    }

    #[test]
    fn indexing_non_array_error() {
        let d = check("program P\nutama { nilai x = 5 tampilkan(x[0]) }");
        assert!(has_code(&d, codes::E_NOT_INDEXABLE));
    }

    #[test]
    fn non_integer_index_error() {
        let d = check("program P\nutama { nilai xs: [i32; 2] = [1i32, 2i32] tampilkan(xs[benar]) }");
        assert!(has_code(&d, codes::E_INVALID_OPERAND));
    }

    #[test]
    fn panjang_on_non_array_error() {
        let d = check("program P\nutama { tampilkan(panjang(5)) }");
        assert!(has_code(&d, codes::E_INVALID_OPERAND));
    }

    #[test]
    fn foreach_on_non_array_error() {
        let d = check("program P\nutama { untuk x pada 5 { tampilkan(x) } }");
        assert!(has_code(&d, codes::E_NOT_ITERABLE));
    }

    #[test]
    fn foreach_binds_element_type() {
        let d = check(
            "program P\nutama { nilai xs: [angka; 2] = [1, 2] ubah s = 0 untuk x pada xs { s = s + x } tampilkan(s) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn string_equality_ok() {
        let d = check("program P\nutama { jika \"a\" == \"a\" { tampilkan(\"y\") } }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn string_ordering_rejected() {
        let d = check("program P\nutama { jika \"a\" < \"b\" { } }");
        assert!(has_code(&d, codes::E_INVALID_OPERAND));
    }

    #[test]
    fn panjang_on_string_ok() {
        let d = check("program P\nutama { tampilkan(panjang(\"halo\")) }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn string_int_equality_mismatch() {
        let d = check("program P\nutama { jika \"a\" == 1 { } }");
        assert!(has_code(&d, codes::E_TYPE_MISMATCH_BINOP));
    }

    #[test]
    fn generic_function_valid() {
        let d = check(
            "program P\n\
             fungsi maksimum<T>(a: T, b: T) -> T { jika a > b { kembalikan a } kembalikan b }\n\
             utama { tampilkan(maksimum(3, 9)) tampilkan(maksimum(2.5, 1.0)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn generic_instantiation_type_error() {
        let d = check(
            "program P\n\
             fungsi maksimum<T>(a: T, b: T) -> T { jika a > b { kembalikan a } kembalikan b }\n\
             utama { tampilkan(maksimum(\"a\", \"b\")) }",
        );
        assert!(has_code(&d, codes::E_INVALID_OPERAND));
    }

    #[test]
    fn generic_arg_type_conflict() {
        let d = check(
            "program P\n\
             fungsi pasangan<T>(a: T, b: T) -> T { kembalikan a }\n\
             utama { nilai x: i32 = 1i32 tampilkan(pasangan(x, 2)) }",
        );
        assert!(d.has_errors());
    }

    #[test]
    fn generic_identity_multiple_types() {
        let d = check(
            "program P\n\
             fungsi id<T>(x: T) -> T { kembalikan x }\n\
             utama { tampilkan(id(1)) tampilkan(id(\"a\")) tampilkan(id(benar)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn valid_ownership_move_and_read() {
        let d = check(
            "program P\n\
             fungsi konsum(k: kotak<i64>) -> i64 { kembalikan isi(k) }\n\
             utama { nilai b = kotak(5) tampilkan(konsum(b)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn use_after_move_rejected() {
        let d = check(
            "program P\n\
             fungsi konsum(k: kotak<i64>) -> i64 { kembalikan isi(k) }\n\
             utama { nilai b = kotak(5) nilai c = b tampilkan(konsum(b)) }",
        );
        assert!(has_code(&d, codes::E_USE_AFTER_MOVE));
    }

    #[test]
    fn move_in_loop_rejected() {
        let d = check(
            "program P\n\
             fungsi konsum(k: kotak<i64>) -> i64 { kembalikan isi(k) }\n\
             utama { nilai b = kotak(5) untuk i dari 0 sampai 3 { tampilkan(konsum(b)) } }",
        );
        assert!(has_code(&d, codes::E_MOVE_IN_LOOP));
    }

    #[test]
    fn inconsistent_move_rejected() {
        let d = check(
            "program P\n\
             fungsi konsum(k: kotak<i64>) -> i64 { kembalikan isi(k) }\n\
             utama { nilai b = kotak(5) nilai c = 1 jika c > 0 { tampilkan(konsum(b)) } }",
        );
        assert!(has_code(&d, codes::E_INCONSISTENT_MOVE));
    }

    #[test]
    fn consistent_move_both_branches_ok() {
        let d = check(
            "program P\n\
             fungsi konsum(k: kotak<i64>) -> i64 { kembalikan isi(k) }\n\
             utama { nilai b = kotak(5) nilai c = 1 jika c > 0 { tampilkan(konsum(b)) } lain { tampilkan(konsum(b)) } }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn owned_in_extern_rejected() {
        let d = check(
            "program P\nekstern fungsi pakai(k: kotak<i64>) -> i64\nutama { }",
        );
        assert!(has_code(&d, codes::E_OWNED_IN_FFI));
    }

    #[test]
    fn owned_reassignment_rejected() {
        let d = check(
            "program P\nutama { ubah b = kotak(1) b = kotak(2) tampilkan(isi(b)) }",
        );
        assert!(has_code(&d, codes::E_OWNED_REASSIGN));
    }

    #[test]
    fn isi_requires_copy_inner() {
        let d = check("program P\nutama { nilai b = kotak(kotak(1)) tampilkan(isi(isi(b))) }");
        assert!(d.has_errors());
    }

    #[test]
    fn owned_struct_moves() {
        let d = check(
            "program P\ntipe Simpul { data: kotak<i64> }\n\
             fungsi pakai(s: Simpul) -> i64 { kembalikan isi(s.data) }\n\
             utama { nilai s = Simpul { data: kotak(9) } nilai t = s tampilkan(pakai(s)) }",
        );
        assert!(has_code(&d, codes::E_USE_AFTER_MOVE));
    }

    #[test]
    fn shared_reference_read_ok() {
        let d = check(
            "program P\nfungsi baca(x: &angka) -> angka { kembalikan *x }\n\
             utama { nilai n = 7 tampilkan(baca(ambil(n))) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn mutable_reference_write_ok() {
        let d = check(
            "program P\nfungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
             utama { ubah n = 41 naik(ambil_ubah(n)) tampilkan(n) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn many_shared_references_coexist() {
        let d = check(
            "program P\nfungsi jumlah(a: &angka, b: &angka) -> angka { kembalikan *a + *b }\n\
             utama { nilai n = 5 tampilkan(jumlah(ambil(n), ambil(n))) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn sequential_mutable_references_ok() {
        let d = check(
            "program P\nfungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
             utama { ubah n = 0 naik(ambil_ubah(n)) naik(ambil_ubah(n)) tampilkan(n) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn borrow_of_non_place_rejected() {
        let d = check(
            "program P\nfungsi baca(x: &angka) -> angka { kembalikan *x }\n\
             utama { tampilkan(baca(ambil(1 + 2))) }",
        );
        assert!(has_code(&d, codes::E_BORROW_NOT_PLACE));
    }

    #[test]
    fn mutable_borrow_of_immutable_rejected() {
        let d = check(
            "program P\nfungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
             utama { nilai n = 5 naik(ambil_ubah(n)) tampilkan(n) }",
        );
        assert!(has_code(&d, codes::E_BORROW_IMMUTABLE));
    }

    #[test]
    fn deref_of_non_reference_rejected() {
        let d = check("program P\nutama { nilai n = 5 tampilkan(*n) }");
        assert!(has_code(&d, codes::E_DEREF_NON_REF));
    }

    #[test]
    fn read_owned_through_reference_rejected() {
        let d = check(
            "program P\nfungsi ambilkotak(b: &kotak<angka>) -> kotak<angka> { kembalikan *b }\n\
             utama { }",
        );
        assert!(has_code(&d, codes::E_DEREF_NON_COPY));
    }

    #[test]
    fn write_through_shared_reference_rejected() {
        let d = check("program P\nfungsi f(x: &angka) { *x = 1 }\nutama { }");
        assert!(has_code(&d, codes::E_STORE_THROUGH_SHARED));
    }

    #[test]
    fn shared_then_mutable_borrow_conflict() {
        let d = check(
            "program P\nutama { ubah n = 5 nilai a = ambil(n) nilai b = ambil_ubah(n) tampilkan(*a) }",
        );
        assert!(has_code(&d, codes::E_BORROW_CONFLICT));
    }

    #[test]
    fn mutable_then_shared_borrow_conflict() {
        let d = check(
            "program P\nutama { ubah n = 5 nilai a = ambil_ubah(n) nilai b = ambil(n) tampilkan(*b) }",
        );
        assert!(has_code(&d, codes::E_BORROW_CONFLICT));
    }

    #[test]
    fn mutate_while_borrowed_rejected() {
        let d = check(
            "program P\nutama { ubah n = 5 nilai r = ambil(n) n = 10 tampilkan(*r) }",
        );
        assert!(has_code(&d, codes::E_MUTATE_WHILE_BORROWED));
    }

    #[test]
    fn returning_reference_rejected() {
        let d = check("program P\nfungsi bocor(n: angka) -> &angka { kembalikan ambil(n) }\nutama { }");
        assert!(has_code(&d, codes::E_REF_ESCAPE));
    }

    #[test]
    fn reference_in_struct_rejected() {
        let d = check(
            "program P\ntipe Bungkus { r: &angka }\n\
             utama { nilai n = 5 nilai w = Bungkus { r: ambil(n) } tampilkan(*w.r) }",
        );
        assert!(has_code(&d, codes::E_REF_ESCAPE));
    }

    #[test]
    fn reference_in_extern_rejected() {
        let d = check("program P\nekstern fungsi c_pakai(x: &angka) -> angka\nutama { }");
        assert!(has_code(&d, codes::E_REF_IN_FFI));
    }

    #[test]
    fn borrow_after_move_rejected() {
        let d = check(
            "program P\nfungsi baca(b: &kotak<angka>) -> angka { kembalikan isi(b) }\n\
             utama { nilai b = kotak(1) nilai c = b tampilkan(baca(ambil(b))) }",
        );
        assert!(has_code(&d, codes::E_USE_AFTER_MOVE));
    }

    #[test]
    fn generic_reference_instantiation_ok() {
        let d = check(
            "program P\nfungsi baca<T>(x: &T) -> T { kembalikan *x }\n\
             utama { nilai n = 3 tampilkan(baca(ambil(n))) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn generic_reference_owned_instantiation_rejected() {
        let d = check(
            "program P\nfungsi baca<T>(x: &T) -> T { kembalikan *x }\n\
             utama { nilai b = kotak(5) tampilkan(isi(baca(ambil(b)))) }",
        );
        assert!(has_code(&d, codes::E_DEREF_NON_COPY));
    }

    #[test]
    fn reference_to_struct_field_ok() {
        let d = check(
            "program P\ntipe Titik { x: angka, y: angka }\n\
             fungsi naik(v: &ubah angka) { *v = *v + 1 }\n\
             utama { ubah p = Titik { x: 1, y: 2 } naik(ambil_ubah(p.x)) tampilkan(p.x) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn reference_to_array_element_ok() {
        let d = check(
            "program P\nfungsi naik(x: &ubah angka) { *x = *x + 1 }\n\
             utama { ubah a = [10, 20, 30] naik(ambil_ubah(a[1])) tampilkan(a[1]) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    const TRAIT_PRELUDE: &str = "program P\n\
        tipe Persegi { sisi: angka }\n\
        sifat Luas { fungsi luas(diri: &Diri) -> angka }\n\
        implementasi Luas untuk Persegi { fungsi luas(diri: &Diri) -> angka { kembalikan diri.sisi * diri.sisi } }\n";

    #[test]
    fn trait_impl_and_method_call_ok() {
        let d = check(&format!(
            "{TRAIT_PRELUDE}utama {{ nilai p = Persegi {{ sisi: 3 }} tampilkan(p.luas()) }}"
        ));
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn inherent_method_ok() {
        let d = check(
            "program P\ntipe Titik { x: angka }\n\
             implementasi Titik { fungsi ambil_x(diri: &Diri) -> angka { kembalikan diri.x } }\n\
             utama { nilai t = Titik { x: 9 } tampilkan(t.ambil_x()) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn mutable_method_ok() {
        let d = check(
            "program P\ntipe Hitung { n: angka }\n\
             implementasi Hitung { fungsi naik(diri: &ubah Diri) { diri.n = diri.n + 1 } }\n\
             utama { ubah h = Hitung { n: 0 } h.naik() tampilkan(h.n) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn trait_bounded_generic_ok() {
        let d = check(&format!(
            "{TRAIT_PRELUDE}\
             fungsi cetak<T: Luas>(v: &T) {{ tampilkan(v.luas()) }}\n\
             utama {{ nilai p = Persegi {{ sisi: 5 }} cetak(ambil(p)) }}"
        ));
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn bound_not_satisfied_rejected() {
        let d = check(&format!(
            "{TRAIT_PRELUDE}tipe Kosong {{ a: angka }}\n\
             fungsi cetak<T: Luas>(v: &T) {{ tampilkan(v.luas()) }}\n\
             utama {{ nilai k = Kosong {{ a: 1 }} cetak(ambil(k)) }}"
        ));
        assert!(has_code(&d, codes::E_BOUND_NOT_SATISFIED));
    }

    #[test]
    fn method_not_found_rejected() {
        let d = check(
            "program P\ntipe A { x: angka }\nutama { nilai a = A { x: 1 } tampilkan(a.hilang()) }",
        );
        assert!(has_code(&d, codes::E_METHOD_NOT_FOUND));
    }

    #[test]
    fn impl_missing_method_rejected() {
        let d = check(
            "program P\ntipe A { x: angka }\n\
             sifat S { fungsi m(diri: &Diri) -> angka }\n\
             implementasi S untuk A { }\nutama { }",
        );
        assert!(has_code(&d, codes::E_IMPL_METHOD_MISMATCH));
    }

    #[test]
    fn impl_signature_mismatch_rejected() {
        let d = check(
            "program P\ntipe A { x: angka }\n\
             sifat S { fungsi m(diri: &Diri) -> angka }\n\
             implementasi S untuk A { fungsi m(diri: &Diri) -> teks { kembalikan \"x\" } }\nutama { }",
        );
        assert!(has_code(&d, codes::E_IMPL_METHOD_MISMATCH));
    }

    #[test]
    fn extra_method_in_trait_impl_rejected() {
        let d = check(
            "program P\ntipe A { x: angka }\n\
             sifat S { fungsi m(diri: &Diri) -> angka }\n\
             implementasi S untuk A { \
               fungsi m(diri: &Diri) -> angka { kembalikan diri.x } \
               fungsi dua(diri: &Diri) -> angka { kembalikan diri.x } }\nutama { }",
        );
        assert!(has_code(&d, codes::E_IMPL_METHOD_MISMATCH));
    }

    #[test]
    fn duplicate_impl_rejected() {
        let d = check(&format!(
            "{TRAIT_PRELUDE}implementasi Luas untuk Persegi {{ fungsi luas(diri: &Diri) -> angka {{ kembalikan 0 }} }}\nutama {{ }}"
        ));
        assert!(has_code(&d, codes::E_DUP_IMPL));
    }

    #[test]
    fn duplicate_trait_method_rejected() {
        let d = check(
            "program P\nsifat S { fungsi m(diri: &Diri) -> angka fungsi m(diri: &Diri) -> angka }\nutama { }",
        );
        assert!(has_code(&d, codes::E_TRAIT_DUP_METHOD));
    }

    #[test]
    fn unknown_trait_in_impl_rejected() {
        let d = check(
            "program P\ntipe A { x: angka }\n\
             implementasi Tak untuk A { fungsi m(diri: &Diri) -> angka { kembalikan diri.x } }\nutama { }",
        );
        assert!(has_code(&d, codes::E_UNKNOWN_TRAIT_OR_TYPE));
    }

    #[test]
    fn ambiguous_method_rejected() {
        let d = check(
            "program P\ntipe A { x: angka }\n\
             sifat S1 { fungsi m(diri: &Diri) -> angka }\n\
             sifat S2 { fungsi m(diri: &Diri) -> angka }\n\
             implementasi S1 untuk A { fungsi m(diri: &Diri) -> angka { kembalikan diri.x } }\n\
             implementasi S2 untuk A { fungsi m(diri: &Diri) -> angka { kembalikan diri.x } }\n\
             utama { nilai a = A { x: 1 } tampilkan(a.m()) }",
        );
        assert!(has_code(&d, codes::E_METHOD_AMBIGUOUS));
    }

    #[test]
    fn diri_outside_impl_rejected() {
        let d = check("program P\nfungsi f(x: &Diri) -> angka { kembalikan 0 }\nutama { }");
        assert!(d.has_errors());
    }

    const HASIL_BAGI: &str = "program P\n\
        fungsi bagi(a: angka, b: angka) -> Hasil<angka, teks> {\n\
            jika b == 0 { kembalikan gagal(\"nol\") }\n\
            kembalikan sukses(a / b)\n\
        }\n";

    #[test]
    fn hasil_construct_and_inspect_ok() {
        let d = check(&format!(
            "{HASIL_BAGI}utama {{ nilai r = bagi(10, 2) \
             jika berhasil(r) {{ tampilkan(nilai_atau(r, 0)) }} \
             lain {{ tampilkan(gagal_atau(r, \"?\")) }} }}"
        ));
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn hasil_propagation_ok() {
        let d = check(&format!(
            "{HASIL_BAGI}fungsi dua(a: angka, b: angka, c: angka) -> Hasil<angka, teks> {{ \
             nilai x = coba bagi(a, b) nilai y = coba bagi(x, c) kembalikan sukses(y) }}\n\
             utama {{ nilai r = dua(100, 5, 2) tampilkan(nilai_atau(r, 0)) }}"
        ));
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn generic_hasil_ok() {
        let d = check(
            "program P\nfungsi bungkus<T>(x: T) -> Hasil<T, teks> { kembalikan sukses(x) }\n\
             utama { nilai a = bungkus(42) tampilkan(nilai_atau(a, 0)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn sukses_without_context_rejected() {
        let d = check("program P\nutama { nilai r = sukses(5) tampilkan(berhasil(r)) }");
        assert!(has_code(&d, codes::E_HASIL_NO_CONTEXT));
    }

    #[test]
    fn coba_on_non_hasil_rejected() {
        let d = check("program P\nutama { nilai x = 5 nilai y = coba x }");
        assert!(has_code(&d, codes::E_TRY_NON_HASIL));
    }

    #[test]
    fn coba_outside_hasil_function_rejected() {
        let d = check(&format!("{HASIL_BAGI}utama {{ nilai y = coba bagi(1, 1) }}"));
        assert!(has_code(&d, codes::E_TRY_BAD_CONTEXT));
    }

    #[test]
    fn coba_error_type_mismatch_rejected() {
        let d = check(&format!(
            "{HASIL_BAGI}fungsi f() -> Hasil<angka, angka> {{ nilai x = coba bagi(1, 1) kembalikan sukses(x) }}\nutama {{ }}"
        ));
        assert!(has_code(&d, codes::E_TRY_BAD_CONTEXT));
    }

    #[test]
    fn hasil_owned_payload_construction_allowed() {
        let d = check(
            "program P\nfungsi f() -> Hasil<kotak<angka>, teks> { kembalikan sukses(kotak(1)) }\nutama { nilai r = f() }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn inspect_non_hasil_rejected() {
        let d = check("program P\nutama { nilai x = 5 tampilkan(berhasil(x)) }");
        assert!(has_code(&d, codes::E_INSPECT_MISUSE));
    }

    #[test]
    fn sukses_type_conflict_rejected() {
        let d = check(
            "program P\nfungsi f() -> Hasil<angka, teks> { kembalikan sukses(\"bukan angka\") }\nutama { }",
        );
        assert!(has_code(&d, codes::E_HASIL_TYPE_CONFLICT));
    }

    #[test]
    fn higher_order_function_ok() {
        let d = check(
            "program P\nfungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
             fungsi terapkan(f: fungsi(angka, angka) -> angka, x: angka, y: angka) -> angka { kembalikan f(x, y) }\n\
             utama { tampilkan(terapkan(tambah, 3, 4)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn function_value_in_variable_ok() {
        let d = check(
            "program P\nfungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
             utama { nilai g = tambah tampilkan(g(1, 2)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn calling_non_function_rejected() {
        let d = check("program P\nutama { nilai x = 5 tampilkan(x(1)) }");
        assert!(has_code(&d, codes::E_NOT_CALLABLE));
    }

    #[test]
    fn function_value_arity_checked() {
        let d = check(
            "program P\nfungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }\n\
             utama { nilai g = tambah tampilkan(g(1)) }",
        );
        assert!(has_code(&d, codes::E_ARITY_MISMATCH));
    }

    #[test]
    fn thread_spawn_and_join_ok() {
        let d = check(
            "program P\nfungsi kerja(n: angka) -> angka { kembalikan n * 2 }\n\
             utama { nilai h = mulai(kerja, 21) tampilkan(tunggu(h)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn thread_worker_must_be_angka_fn() {
        let d = check(
            "program P\nfungsi kerja(s: teks) -> angka { kembalikan 0 }\n\
             utama { nilai h = mulai(kerja, 1) }",
        );
        assert!(has_code(&d, codes::E_THREAD_MISUSE));
    }

    #[test]
    fn tunggu_on_non_thread_rejected() {
        let d = check("program P\nutama { nilai x = 5 tampilkan(tunggu(x)) }");
        assert!(has_code(&d, codes::E_THREAD_MISUSE));
    }

    #[test]
    fn daftar_build_and_read_ok() {
        let d = check(
            "program P\nutama { ubah d: daftar<angka> = daftar() \
             dorong(ambil_ubah(d), 1) dorong(ambil_ubah(d), 2) \
             atur(ambil_ubah(d), 0, 9) \
             tampilkan(elemen(ambil(d), 0)) tampilkan(panjang(ambil(d))) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn daftar_without_context_rejected() {
        let d = check("program P\nutama { ubah d = daftar() }");
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn daftar_wrong_element_type_rejected() {
        let d = check(
            "program P\nutama { ubah d: daftar<angka> = daftar() dorong(ambil_ubah(d), \"x\") }",
        );
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn elemen_on_non_daftar_rejected() {
        let d = check("program P\nutama { nilai x = 5 tampilkan(elemen(ambil(x), 0)) }");
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn daftar_owned_element_rejected() {
        let d = check("program P\nutama { ubah d: daftar<kotak<angka>> = daftar() }");
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn numeric_cast_ok() {
        let d = check(
            "program P\nutama { nilai a = 300 nilai b = a sebagai bita tampilkan(b) \
             nilai c = 3 sebagai f64 tampilkan(c) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn cast_non_numeric_rejected() {
        let d = check("program P\nutama { nilai s = \"x\" nilai n = s sebagai angka }");
        assert!(has_code(&d, codes::E_INVALID_CAST));
    }

    #[test]
    fn byte_string_building_ok() {
        let d = check(
            "program P\nutama { ubah b: daftar<bita> = daftar() \
             dorong(ambil_ubah(b), bita_pada(\"hi\", 0)) \
             dorong(ambil_ubah(b), (65 + 1) sebagai bita) \
             cetak_bita(ambil(b)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn bita_pada_requires_teks() {
        let d = check("program P\nutama { nilai n = 5 tampilkan(bita_pada(n, 0)) }");
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn file_io_ok() {
        let d = check(
            "program P\nutama { ubah b: daftar<bita> = daftar() \
             nilai w = tulis_file(\"x\", ambil(b)) \
             nilai r = baca_file(\"x\", ambil_ubah(b)) tampilkan(w) tampilkan(r) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn baca_file_needs_mutable_buffer() {
        let d = check(
            "program P\nutama { ubah b: daftar<bita> = daftar() nilai r = baca_file(\"x\", ambil(b)) }",
        );
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn opaque_pointer_ffi_ok() {
        let d = check(
            "program P\n\
             ekstern fungsi fopen(nama: teks, mode: teks) -> pointer\n\
             ekstern fungsi fclose(f: pointer) -> i32\n\
             utama { nilai f = fopen(\"x\", \"r\") jika f == kosong() { } lain { nilai c = fclose(f) tampilkan(c) } }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn pointer_cannot_be_dereferenced() {
        let d = check(
            "program P\n\
             ekstern fungsi fopen(nama: teks, mode: teks) -> pointer\n\
             utama { nilai f = fopen(\"x\", \"r\") tampilkan(*f) }",
        );
        assert!(has_code(&d, codes::E_DEREF_NON_REF));
    }

    #[test]
    fn pointer_cast_rejected() {
        let d = check("program P\nutama { nilai p = 5 sebagai pointer tampilkan(1) }");
        assert!(has_code(&d, codes::E_INVALID_CAST));
    }

    #[test]
    fn pointer_is_copy_not_owned() {
        let d = check(
            "program P\n\
             ekstern fungsi fopen(nama: teks, mode: teks) -> pointer\n\
             ekstern fungsi fclose(f: pointer) -> i32\n\
             fungsi pakai(p: pointer) -> i32 { kembalikan fclose(p) }\n\
             utama { nilai f = fopen(\"x\", \"r\") nilai a = pakai(f) nilai b = pakai(f) tampilkan(a + b) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn cocok_exhaustive_ok() {
        let d = check(
            "program P\nenum W { A B C }\n\
             fungsi f(w: W) -> angka { cocok w { A => { kembalikan 1 } B => { kembalikan 2 } C => { kembalikan 3 } } kembalikan 0 }\n\
             utama { tampilkan(f(W.A)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn cocok_non_exhaustive_errors() {
        let d = check(
            "program P\nenum W { A B C }\n\
             fungsi f(w: W) { cocok w { A => { } B => { } } }\n\
             utama { f(W.A) }",
        );
        assert!(has_code(&d, codes::E_NON_EXHAUSTIVE_MATCH));
    }

    #[test]
    fn cocok_default_is_exhaustive() {
        let d = check(
            "program P\nenum W { A B C }\n\
             fungsi f(w: W) { cocok w { A => { } lain => { } } }\n\
             utama { f(W.A) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn cocok_unknown_variant_errors() {
        let d = check(
            "program P\nenum W { A B }\n\
             fungsi f(w: W) { cocok w { A => { } Z => { } B => { } } }\n\
             utama { f(W.A) }",
        );
        assert!(has_code(&d, codes::E_UNKNOWN_VARIANT));
    }

    #[test]
    fn cocok_non_enum_errors() {
        let d = check("program P\nutama { cocok 5 { A => { } } }");
        assert!(has_code(&d, codes::E_INVALID_OPERAND));
    }

    #[test]
    fn peta_ok() {
        let d = check(
            "program P\nutama { ubah m: peta<teks, angka> = peta() \
             taruh(ambil_ubah(m), \"a\", 1) \
             nilai v = ambil_peta(ambil(m), \"a\", 0) \
             jika ada(ambil(m), \"a\") { } tampilkan(v) tampilkan(panjang(ambil(m))) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn peta_non_teks_key_errors() {
        let d = check(
            "program P\nutama { ubah m: peta<angka, angka> = peta() taruh(ambil_ubah(m), 1, 2) }",
        );
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn peta_owned_value_errors() {
        let d = check("program P\nutama { ubah m: peta<teks, daftar<angka>> = peta() tampilkan(1) }");
        assert!(has_code(&d, codes::E_COLLECTION_MISUSE));
    }

    #[test]
    fn owned_hasil_construction_ok() {
        let d = check(
            "program P\n\
             fungsi f(b: boolean) -> Hasil<daftar<angka>, teks> {\n\
               jika bukan b { kembalikan gagal(\"e\") }\n\
               ubah x: daftar<angka> = daftar()\n\
               kembalikan sukses(x)\n\
             }\n\
             fungsi g() -> Hasil<angka, teks> { nilai d = coba f(benar) kembalikan sukses(panjang(ambil(d))) }\n\
             utama { nilai r = g() }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
    }

    #[test]
    fn owned_hasil_inspection_rejected() {
        let d = check(
            "program P\n\
             fungsi f() -> Hasil<daftar<angka>, teks> { kembalikan gagal(\"e\") }\n\
             utama { ubah c: daftar<angka> = daftar() nilai r = nilai_atau(f(), c) }",
        );
        assert!(has_code(&d, codes::E_HASIL_NOT_COPY));
    }

    #[test]
    fn owned_hasil_use_after_move() {
        let d = check(
            "program P\n\
             fungsi f() -> Hasil<daftar<angka>, teks> { ubah x: daftar<angka> = daftar() kembalikan sukses(x) }\n\
             fungsi g() -> Hasil<angka, teks> { nilai r = f() nilai a = coba r nilai b = coba r kembalikan sukses(1) }\n\
             utama { nilai x = g() }",
        );
        assert!(has_code(&d, codes::E_USE_AFTER_MOVE));
    }

    #[test]
    fn peta_moves_like_owned() {
        let d = check(
            "program P\nfungsi konsumsi(m: peta<teks, angka>) -> angka { kembalikan panjang(ambil(m)) }\n\
             utama { ubah m: peta<teks, angka> = peta() nilai n = konsumsi(m) tampilkan(panjang(ambil(m))) }",
        );
        assert!(has_code(&d, codes::E_USE_AFTER_MOVE));
    }

    #[test]
    fn daftar_moves_like_owned() {
        let d = check(
            "program P\nfungsi konsumsi(d: daftar<angka>) -> angka { kembalikan panjang(ambil(d)) }\n\
             utama { ubah d: daftar<angka> = daftar() nilai n = konsumsi(d) tampilkan(panjang(ambil(d))) }",
        );
        assert!(has_code(&d, codes::E_USE_AFTER_MOVE));
    }
}
