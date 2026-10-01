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
- ✅ Dynamic lists `daftar<T>` (Phase 16): growable heap-backed vector,
  `daftar()`/`dorong`/`elemen`/`atur`/`panjang`, `realloc` growth, runtime
  bounds checks, Owned (buffer freed on drop), moves. Copy element types for now
- ⬜ Dynamically sized array *slices* (`[T]` views); maps/sets
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
- 🟡 String model: UTF-8 immutable `teks` with content comparison (`strcmp`) and
  byte length; **dynamic text** via `daftar<bita>` byte buffers — `bita_pada`
  (read), `cetak_bita` (write), and the Lime-written `Teks` module
  (`tambah_teks`/`tambah_angka`/…) for building strings. Numeric/char casts via
  `sebagai`. Heap-owned `teks` values and slicing are follow-ups
- 🟡 `lime/std` (Phase 14): standard library written in Lime and embedded in the
  compiler, pulled in with `gunakan`. Shipping now: `Inti` (maks/min/mutlak/
  batas/tanda) and `Matematika` (pangkat/fpb/kpk/apakah_prima/faktorial). io,
  string, collections, filesystem, process, time await further runtime features

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
- ✅ First-class function values (`fungsi(A) -> R` type): functions passed,
  stored, returned, and called indirectly; bare function pointers, no allocation.
  Enables higher-order functions, and is the prerequisite for thread spawning and
  FFI callbacks
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
- ✅ Concurrency (Phase 15): real OS threads via `pthread` — `mulai(f, n)` spawns
  a `fungsi(angka) -> angka` worker, `tunggu(h)` joins and returns its result
  (`Utas<R>` handle). Data-race-free by construction (value passed in, result
  through join, no shared mutable state); zero-trampoline (x86-64 ABI). Richer
  payloads, channels, and Windows threads are follow-ups

## Phase 6 — Developer tooling
- ✅ Formatter core (`lime fmt`, parse-based)
- ⬜ Language server (LSP) + VS Code extension

## Phase 7 — Package ecosystem
- ⬜ `lime.toml` manifest, dependency resolution, package cache, build profiles

## Phase 8 — Optimization
- ✅ Lime IR passes: constant folding + propagation, constant-branch folding,
  dead-code elimination — run to a fixpoint, gated by optimization level (`-O0`
  skips them). The optimizing backend is LLVM at the chosen `-O0..-O3`, so the IR
  passes normalize/shrink rather than duplicate LLVM's inlining, loop and
  load/store optimizations
- ⬜ Escape analysis, LTO, Lime-level inlining where it beats LLVM
- ⬜ Benchmark suite (compile time, size, runtime, allocations, startup)

## Current milestone summary

The bootstrap compiler is a **complete vertical slice**: it lexes, parses,
type-checks, lowers to a Lime-owned SSA IR, optimizes, emits LLVM IR, and links a
**real native executable** that runs and produces correct output — for the
numeric/function/control-flow core of the language. Everything above marked ✅ is
implemented, tested, and exercised by a program that actually runs.
