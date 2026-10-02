# Exotickik Roadmap

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
- ✅ Source map + spans (`exotickik_span`)
- ✅ Diagnostics engine with snippets & codes (`exotickik_diagnostics`)
- ✅ Lexer (`exotickik_lexer`)
- ✅ AST (`exotickik_ast`)
- ✅ Parser (`exotickik_parser`)
- ✅ CLI: `exotickik check | build | run | fmt | version | help`
- ✅ `exotickik check hello.exo` parses & validates

## Phase 2 — Semantic system
- ✅ Symbol tables & scopes
- ✅ Type system core (`exotickik_types`)
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
- ✅ Exotickik IR (SSA) + lowering (`exotickik_ir`)
- ✅ LLVM IR text emission (`exotickik_codegen`)
- ✅ `llc` + linker invocation → native ELF executable
- ✅ `exotickik build hello.exo` produces a real executable; `exotickik run` runs it
- ✅ Validated: hello world, arithmetic, functions, recursion, conditionals, loops
- 🟡 Windows x64 target via `lld-link` (wired, unverified in CI env)

## Phase 4 — Runtime + standard library
- 🟡 `tampilkan` I/O builtin (implemented via libc FFI)
- 🟡 String model: UTF-8 immutable `teks` with content comparison (`strcmp`) and
  byte length; **dynamic text** via `daftar<bita>` byte buffers — `bita_pada`
  (read), `cetak_bita` (write), and the Exotickik-written `Teks` module
  (`tambah_teks`/`tambah_angka`/…) for building strings. Numeric/char casts via
  `sebagai`. Heap-owned `teks` values and slicing are follow-ups
- ✅ File I/O (Phase 18): `baca_file`/`tulis_file` whole-file read/write over
  `daftar<bita>` byte buffers, lowered to libc `fopen`/`fread`/`fwrite`/`fclose`
  (FILE* carried as an opaque i64 across the native ABI), `boolean` result with
  data delivered through a mutable buffer. Paths, streaming, directory iteration
  and richer error values are follow-ups
- 🟡 `exotickik/std` (Phase 14): standard library written in Exotickik and embedded in the
  compiler, pulled in with `gunakan`. Shipping now: `Inti` (maks/min/mutlak/
  batas/tanda) and `Matematika` (pangkat/fpb/kpk/apakah_prima/faktorial). io,
  string, collections, process, time await further runtime features

## Phase 5 — Advanced language
- ✅ Enums (C-style tagged variants: construction, equality, print, params,
  returns, struct fields; native i32-tag codegen)
- ✅ Pattern matching `cocok` (Phase 25, extended Phase 31): statement matching
  on enum variants with compile-time exhaustiveness checking (`E0268`) and an
  optional `lain` default; arms are blocks, lowered to tag-dispatch branches.
  Variants now carry **associated data** and arms **bind** that payload
  (`Angka(n) => { … }`)
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
- ✅ Generic structs and enums (monomorphized); ⬜ explicit type args (turbofish),
  dynamic dispatch / trait objects
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
- ✅ FFI expansion (Phase 21): callbacks (a Exotickik function passed where C expects a
  function pointer, e.g. `atexit`), and the opaque `pointer` type for C handles
  (`FILE*`, allocator handles, …) with the `kosong()` null constructor. `pointer`
  is Copy and FFI-safe but opaque — no deref, no int↔pointer cast, not printable —
  so memory safety is preserved. Owned types and references still may not cross FFI
- ✅ Concurrency (Phase 15): real OS threads via `pthread` — `mulai(f, n)` spawns
  a `fungsi(angka) -> angka` worker, `tunggu(h)` joins and returns its result
  (`Utas<R>` handle). Data-race-free by construction (value passed in, result
  through join, no shared mutable state); zero-trampoline (x86-64 ABI). Richer
  payloads, channels, and Windows threads are follow-ups

## Phase 6 — Developer tooling
- ✅ Formatter core (`exotickik fmt`, parse-based)
- ✅ Formatter CLI (Phase 23): `--check` (CI mode, exits non-zero on unformatted
  files without writing), multi-file and recursive directory formatting, and
  project-wide formatting from `exotickik.toml`; `--write` only rewrites changed files
- ✅ Test runner (`exotickik test`, Phase 19): discovers `uji_*` functions (no params,
  `boolean` return), synthesizes a harness `utama`, compiles and runs them as one
  native executable, flushes per test so a crash still reports prior results,
  exits non-zero on any failure
- ⬜ Language server (LSP) + VS Code extension

## Phase 7 — Package ecosystem
- ✅ Project manifest (`exotickik.toml`, Phase 22): `exotickik init` scaffolds a project;
  `build`/`run`/`check`/`test`/`bench` with no file argument resolve the entry,
  target, and optimization level from the nearest `exotickik.toml` (searched up the
  directory tree). CLI flags override the manifest; an explicit file bypasses it
- ⬜ Dependency resolution, package cache, build profiles, registry (need
  network/registry infrastructure the bootstrap does not ship)

## Phase 8 — Optimization
- ✅ Exotickik IR passes: constant folding + propagation, constant-branch folding,
  dead-code elimination — run to a fixpoint, gated by optimization level (`-O0`
  skips them). The optimizing backend is LLVM at the chosen `-O0..-O3`, so the IR
  passes normalize/shrink rather than duplicate LLVM's inlining, loop and
  load/store optimizations
- ⬜ Escape analysis, LTO, Exotickik-level inlining where it beats LLVM
- ✅ Benchmark runner (`exotickik bench`, Phase 20): discovers `tolok_*` functions (no
  params, `angka` return), times a fixed iteration loop with libc `clock()`,
  sums returns into a checksum to defeat dead-code elimination, reports total
  time and ns/op; `--iterasi N` sets the count. A broader suite (compile time,
  binary size, allocations, startup) is a follow-up

## Phase 9 — Self-hosting
- 🟡 Compiler-shaped components written in Exotickik: an inline-evaluating calculator
  (Phase 24, `examples/kalkulator/`) and a **two-phase AST compiler** (Phase 26,
  `examples/pohon/`) — a lexer, a recursive-descent parser that builds a real
  **arena AST** (`daftar<Node>`, children by index), and a tree-walking evaluator
  dispatching on node kind with `cocok`. Proves Exotickik can build and traverse a real
  AST today; each is a `modul` with a demo and a `exotickik test` suite
- ✅ Enum associated data (sum types, Phase 31): variants carry positional
  payloads (`Token { Angka(angka) Operator(char) Akhir }`), constructed with
  `Token.Angka(10)` and destructured in `cocok` by binding (`Angka(n) => …`).
  Represented as a tagged aggregate (`{ i32 tag, flattened payload fields }`),
  reusing the struct/`Hasil` aggregate codegen with no runtime indirection. An
  AST node can now carry its data inline instead of via the arena `Node` struct.
  Copy payloads for now; owned payloads (`daftar`, `kotak`, `peta`) and generic
  enums are the next step
- ⬜ Heap-recursive data types (nullable box / optional variant). Not required for
  an AST — the arena approach works today — but would enable pointer-based trees
- ✅ `peta<K, V>` hash map (Phase 27): real open-addressing table (polynomial
  string hash, linear probing, automatic growth + rehash), `peta`/`taruh`/
  `ambil_peta`/`ada`/`panjang`, Owned with deterministic buffer cleanup, usable by
  reference. `teks` keys (symbol tables, interning, module tables); integer keys a
  follow-up
- 🟡 Owned/dynamic strings (Phase 28): `daftar<bita>` is the owned, growable,
  mutable string (owned storage, length, indexing, deterministic cleanup); the
  `Teks` module now adds comparison (`sama_teks`/`sama_bita`), search (`cari_bita`),
  prefix (`awalan`), and slicing (`potong`) — the lexer string ops. A first-class
  owned `teks` value type and a buffer→`teks` conversion remain
- ✅ `Hasil<T, R>` with owned payloads (Phase 29): `T`/`R` may be Owned
  (`daftar`, `peta`, `kotak`, owned buffers); the result moves and drops only its
  active payload (tag-conditional drop), `coba` moves the payload out or propagates
  the error by move. Inspection builtins stay Copy-only (use `coba`). Valgrind-clean
- ✅ Generic structs (Phase 30): `tipe Nama<T, U> { … }` with monomorphization to
  concrete structs (no runtime cost), type args from the expected type or inferred
  from field values, nesting (`Pasangan<Titik<angka>, angka>`, `daftar<Titik<T>>`),
  and ownership following the concrete fields (owned field ⇒ Owned struct). Reuses
  all existing struct IR/codegen
- ✅ Enum associated data + `cocok` payload binding (Phase 31): Copy payloads,
  tagged-aggregate representation. Owned payloads (which also unblock
  owned-`Hasil` inspection) and generic enums (`Pilihan<T>`) are the next step
- ✅ Generic enums (Phase 32): `enum Pilihan<T> { Ada(T) Kosong }`, monomorphized
  to concrete enums like generic structs; instantiation from the expected type or
  inferred from a variant's payload arguments (`Pilihan.Ada(5)` ⇒ `Pilihan<angka>`).
  Reuses the enum associated-data representation
- ✅ Owned enum payloads (Phase 33): a variant may carry owned data (`daftar`,
  `kotak`, `peta`, owned structs/enums, owned generic type args). An enum with an
  owned payload is Owned — construction moves the payload in, the value moves,
  and it drops by tag-conditional freeing of only the active variant's payload
  (sharing the owned-`Hasil` machinery). `cocok` consumes an owned scrutinee;
  binding an arm takes ownership of the payload, an unbound/`lain` arm drops the
  active payload — released exactly once. Valgrind-clean (no leak, no
  double-free). `daftar` with owned element types remains a separate follow-up
- 🟡 Port the bootstrap front end to Exotickik component by component (`compiler/`),
  validated with `exotickik test` and differential tests against the Rust bootstrap.
  Done: source/span infrastructure (`Rentang`, `Sumber`), token model (`Token`),
  and a **real self-hosted lexer** (`Lekser`) for the actual Exotickik language —
  differential-tested token-for-token against the bootstrap lexer
  (`cli/tests/selfhost_lexer.rs`). Next: the parser → AST

  See `docs/SELF_HOSTING.md` for the evidence-based gap analysis.

## Current milestone summary

The bootstrap compiler is a **complete vertical slice**: it lexes, parses,
type-checks, lowers to a Exotickik-owned SSA IR, optimizes, emits LLVM IR, and links a
**real native executable** that runs and produces correct output — for the
numeric/function/control-flow core of the language. Everything above marked ✅ is
implemented, tested, and exercised by a program that actually runs.
