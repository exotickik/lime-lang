# Phase 24 (First self-hosting component) — Design Assessment

Baseline: `main` after formatter tooling (Phase 23). Ninth roadmap item:
everything else required for self-hosting. Self-hosting the whole compiler is a
large, multi-phase effort; this phase makes a concrete, honest first step and
produces an evidence-based map of what remains.

## Goal

Write a real, compiler-shaped program **entirely in Lime**, run and test it
through the normal toolchain, and from that experience identify the specific
language gaps that still block a self-hosted Lime compiler — rather than claiming
self-hosting prematurely.

## What was built

An arithmetic expression evaluator, structured as a project so it also exercises
modules and the test runner:

- `examples/kalkulator/Kalkulator.lime` — `modul Kalkulator`, `publik fungsi
  evaluasi(teks) -> angka`. A byte-scanning lexer builds a `daftar<Token>` (a
  `tipe Token { jenis, muatan }`), and a precedence-climbing evaluator recurses
  over `&daftar<Token>`, threading position/value through a Copy `Langkah` struct.
  Handles `+ - * /`, parentheses, precedence, and left-associativity.
- `examples/kalkulator/main.lime` — demo entry (`impor Kalkulator`).
- `examples/kalkulator/uji.lime` — six `uji_*` tests run by `lime test`.
- `cli/tests/selfhost.rs` — pins the demo output and the test-suite result.

This deliberately uses only shipped features: structs, `daftar<T>`, references
into functions, recursion, byte scanning, casts, modules, `lime test`, `lime fmt`.

## Why an evaluator and not a parser-to-AST

A full compiler needs an AST (a recursive, tagged tree). Precedence climbing
evaluates **inline** without materializing a tree, so it was expressible today and
makes a clean proof that the non-AST parts of a compiler — lexing, token storage,
recursive descent with precedence — already work in Lime. Building the tree is
exactly what surfaced the remaining gaps.

## Evidence-based gaps (verified against the compiler)

1. **Enum associated data + `cocok`.** `enum Pohon { Daun(angka) Cabang }` fails to
   parse (`E0110`). No payload-carrying variants and no pattern matching → an
   AST/IR node (a tagged union) cannot be represented idiomatically. Biggest
   blocker.
2. **Constructible recursive types.** `tipe Simpul { nilai_x: angka, berikut:
   kotak<Simpul> }` *type-checks*, but `kotak` is non-nullable and there is no
   optional/variant to terminate recursion, so finite lists/trees cannot be
   built. Needs gap 1 or a nullable box.
3. **Generic structs/enums.** `tipe Pasangan<T> { … }` fails to parse; only
   generic *functions* exist today. Reusable typed containers need this.
4. **Maps/sets and owned strings.** Symbol tables need `peta<K, V>`; diagnostics
   and identifiers need owned `teks` and `Hasil` with owned payloads. Only
   `daftar<T>` and byte-buffer text exist now.

Full detail and the suggested order to close them is in `docs/SELF_HOSTING.md`.

## Tests

- `cli/tests/selfhost.rs`: the Lime-written calculator evaluates five expressions
  to the correct values and passes its own six-test `lime test` suite.
- `examples/kalkulator/uji.lime`: the in-language test suite.

## Honest status

This is **not** a self-hosted compiler. It is the first real compiler component in
Lime plus a verified gap analysis. The calculator is executable proof that
lexing, token collections, and recursive parsing are expressible today; the AST/IR
core awaits sum types, recursive data, generic types, and maps, which are now
explicit roadmap items (Phase 9).
