# Lime Roadmap

Legend: ✅ done · 🟡 in progress · ⬜ planned

## Phase 0 — Language design
- ✅ Language identity & spec (`LANGUAGE_SPEC.md`)
- ✅ Grammar (`GRAMMAR.md`)
- ✅ Architecture (`ARCHITECTURE.md`)
- ✅ Memory model (`MEMORY_MODEL.md`)
- ✅ Type system (`TYPE_SYSTEM.md`)
- ✅ This roadmap

## Phase 1 — Compiler foundation
- ✅ Cargo workspace / build system
- ✅ Source map + spans (`lime_span`)
- ✅ Diagnostics engine with snippets & codes (`lime_diagnostics`)
- ✅ Lexer (`lime_lexer`)
- ✅ AST (`lime_ast`)
- ✅ Parser (`lime_parser`)
- ✅ CLI: `lime check | build | run | fmt | version | help`
- ✅ `lime check hello.lime` parses & validates

## Phase 2 — Semantic system
- ✅ Symbol tables & scopes
- ✅ Type system core (`lime_types`)
- ✅ Type checking: functions, bindings, expressions, calls, control flow
- ✅ Control-flow / return checking
- ✅ Structs: declaration, literals, field access, nested fields, field
  mutation, by-value params/returns, native aggregate codegen
- ✅ Fixed-size arrays `[T; N]`: literals, indexing, mutation, `panjang`,
  for-each (`pada`) and index iteration, runtime bounds checking, arrays of
  structs / arrays as params & returns
- ⬜ Dynamically sized arrays / slices (`[T]` views)
- ✅ Ownership model (Phase 8): `salinan`/`milik` classification, `kotak<T>`
  owned heap cell, move semantics, use-after-move detection, branch-consistent
  moves, deterministic drop (malloc/free), recursive drop of nested owned
  values, ownership through calls/returns, generics & FFI boundaries
- ✅ References `&`/`&ubah` with borrow checking (Phase 9): `ambil`/`ambil_ubah`
  borrows, `*r` read / `*r = e` write, shared-vs-mutable aliasing rules, lexical
  non-escaping loans (no dangling without lifetime annotations), mutation through
  native pointers (`load`/`store`), references over struct fields / array
  elements / generics, and FFI rejection

## Phase 3 — Native backend
- ✅ Lime IR (SSA) + lowering (`lime_ir`)
- ✅ LLVM IR text emission (`lime_codegen`)
- ✅ `llc` + linker invocation → native ELF executable
- ✅ `lime build hello.lime` produces a real executable; `lime run` runs it
- ✅ Validated: hello world, arithmetic, functions, recursion, conditionals, loops
- 🟡 Windows x64 target via `lld-link` (wired, unverified in CI env)

## Phase 4 — Runtime + standard library
- 🟡 `tampilkan` I/O builtin (implemented via libc FFI)
- 🟡 String model: UTF-8, immutable value semantics, content comparison
  (`strcmp`), byte length (`panjang`/`strlen`); concatenation, slicing,
  indexing, and owned strings pending the allocator
- ⬜ `lime/std`: io, string, collections, filesystem, process, math, time

## Phase 5 — Advanced language
- ✅ Enums (C-style tagged variants: construction, equality, print, params,
  returns, struct fields; native i32-tag codegen)
- ⬜ Enum associated data + pattern matching (`cocok`)
- ✅ Generic functions with type inference and monomorphization (native
  specialization, instantiation-time checking, nested generic calls)
- ✅ Traits (`sifat`/`implementasi`, Phase 11): trait & inherent methods, method
  calls with `&Diri`/`&ubah Diri`/`Diri` receivers, static dispatch via
  monomorphization, trait-bounded generics (`<T: Sifat>`), impl conformance and
  bound checking, real native lowering (methods are ordinary functions, no
  vtables)
- ⬜ Generic structs/enums, explicit type args, dynamic dispatch / trait objects
- ✅ Error handling (`Hasil<T, R>`, Phase 12): built-in result type, `sukses`/
  `gagal` constructors, `coba` propagation (early-return), `berhasil`/
  `nilai_atau`/`gagal_atau` inspection, generic integration, native tagged-value
  lowering (no exceptions, no allocation). Owned payloads and `cocok`
  destructuring are documented follow-ups
- ✅ Modules across files (Phase 13): `modul` library files, `impor <Nama>`
  transitive loading with cycle detection, `publik` visibility enforced across
  module boundaries (functions/structs/enums/traits), multi-file native builds.
  Flat namespace for now; qualified names are a follow-up
- ✅ C-ABI FFI (`ekstern fungsi`, including variadic; links against libc) —
  see `docs/FFI.md`
- ⬜ Concurrency

## Phase 6 — Developer tooling
- ✅ Formatter core (`lime fmt`, parse-based)
- ⬜ Language server (LSP) + VS Code extension

## Phase 7 — Package ecosystem
- ⬜ `lime.toml` manifest, dependency resolution, package cache, build profiles

## Phase 8 — Optimization
- ✅ Constant folding, dead-code elimination (Lime IR passes)
- ⬜ Inlining, loop opts, escape analysis, LTO
- ⬜ Benchmark suite (compile time, size, runtime, allocations, startup)

## Current milestone summary

The bootstrap compiler is a **complete vertical slice**: it lexes, parses,
type-checks, lowers to a Lime-owned SSA IR, optimizes, emits LLVM IR, and links a
**real native executable** that runs and produces correct output — for the
numeric/function/control-flow core of the language. Everything above marked ✅ is
implemented, tested, and exercised by a program that actually runs.
