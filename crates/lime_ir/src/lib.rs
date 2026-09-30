pub mod ir;
pub mod lower;
pub mod opt;

pub use ir::*;
pub use lower::lower;
pub use opt::{optimize, optimize_level};

#[cfg(test)]
mod tests {
    use super::*;
    use lime_diagnostics::DiagnosticEngine;
    use lime_lexer::tokenize;
    use lime_parser::parse;
    use lime_span::SourceMap;

    fn build(src: &str) -> (Module, DiagnosticEngine) {
        let mut sm = SourceMap::new();
        let f = sm.add("t.lime", src);
        let mut diags = DiagnosticEngine::new();
        let toks = tokenize(f, src, &mut diags);
        let prog = parse(f, toks, &mut diags);
        let results = lime_sema::analyze(&prog, &mut diags);
        let module = lower(&prog, &results, &mut diags);
        (module, diags)
    }

    #[test]
    fn lowers_main() {
        let (m, d) = build("program P\nutama { tampilkan(\"hai\") }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert!(m.funcs.iter().any(|f| f.is_main));
        assert!(m.needs_printf);
    }

    #[test]
    fn lowers_function_and_call() {
        let (m, d) = build(
            "program P\nfungsi f(a: angka) -> angka { kembalikan a + 1 }\nutama { tampilkan(f(2)) }",
        );
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert!(m.funcs.iter().any(|f| f.name == "f"));
    }

    #[test]
    fn constant_folding_reduces_add() {
        let (mut m, d) = build("program P\nutama { nilai x = 1 + 2 * 3 tampilkan(x) }");
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        optimize(&mut m);
        let main = m.funcs.iter().find(|f| f.is_main).unwrap();
        let has_store_const_7 = main.blocks.iter().any(|b| {
            b.instrs.iter().any(|i| {
                matches!(&i.kind, InstrKind::Store { value: Operand::ConstInt(7, _), .. })
            })
        });
        assert!(has_store_const_7, "expected folded constant 7 stored");
    }

    #[test]
    fn dce_removes_unused_pure_instr() {
        let (mut m, _d) = build("program P\nutama { nilai x = 5 + 5 tampilkan(1) }");
        optimize(&mut m);
        let main = m.funcs.iter().find(|f| f.is_main).unwrap();
        let bin_count: usize = main
            .blocks
            .iter()
            .map(|b| b.instrs.iter().filter(|i| matches!(i.kind, InstrKind::Bin { .. })).count())
            .sum();
        assert_eq!(bin_count, 0, "unused arithmetic should be eliminated");
    }
}
