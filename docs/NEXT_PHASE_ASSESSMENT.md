# Phase 8 (Memory Model / Ownership) — Implementation Assessment

Baseline before this phase: `cargo build --workspace` and `cargo test --workspace`
both green (125 tests, 0 warnings).

## Current memory semantics (as actually implemented)

The compiler today has **one uniform memory model: everything is a copyable
value on the stack, and nothing is heap-allocated at runtime.**

- **Primitives** (`i8..i64`, `u8..u64`, `f32`, `f64`, `boolean`, `karakter`,
  `hampa`, enum tags) are register/stack values, copied on assignment, argument
  passing, and return.
- **`teks`** is a pointer to a **static, deduplicated, NUL-terminated** UTF-8
  buffer. It is immutable and freely copyable (a pointer copy). No string is ever
  heap-allocated, so a `teks` owns nothing.
- **Structs, arrays, enums** are inline LLVM aggregates. Binding, passing, and
  returning **copy** the whole aggregate; `llc -O2` (mem2reg/SROA) promotes these
  to registers. Field/element access uses `extractvalue` / `getelementptr`.
- **Generics** monomorphize; each instance is an ordinary concrete function, so
  it inherits the same value semantics.
- There are **no references, no moves, no destructors, and no `free`**. Assigning
  to an `ubah` binding overwrites in place; because no binding owns a heap
  resource, there is nothing to clean up. The current model is trivially sound:
  no aliasing (everything copies) and no leaks/double-frees (no heap).

Conclusion: the existing types are all **Copy** types. Ownership only becomes
meaningful once a type exists that *owns a heap allocation*. None does today.

## What must change

To give ownership real semantics without disturbing the working language:

1. Introduce an **owned heap type** so that move/drop have observable meaning:
   `kotak<T>` — an owned, heap-allocated cell holding one `T`.
2. Classify every type as **`salinan` (Copy)** or **`milik` (Owned)**. All
   existing types stay Copy; only `kotak<T>` (and any aggregate that transitively
   contains one) is Owned. This preserves every existing test and example.
3. Add **move semantics** for Owned values (assignment, argument, return, literal
   field/element, `kotak(...)`), with **use-after-move** detection.
4. Add **deterministic drop**: at scope exit and before every `kembalikan` /
   `berhenti` / `lanjut`, free each still-owning Owned local (recursively for
   aggregates), never a moved one, never twice.
5. Add **real heap runtime**: `malloc`/`free` through the existing C-ABI FFI (no
   fake allocator).
6. Extend **Lime IR** with allocation and drop operations that lower to real
   native `malloc`/`free`, and thread ownership results from sema into lowering.

Nothing about the Copy path changes, so structs, enums, arrays, strings,
generics, FFI, the formatter, the CLI, and all examples keep working unchanged.

## Design decisions (Lime's own model, not Rust's)

- **Two categories, inferred not annotated.** A type is Owned iff it is `kotak<T>`
  or an aggregate transitively containing an Owned type; otherwise Copy. There is
  no `Copy`-trait opt-in and no move/copy sigil at use sites — the category is a
  property of the type, decided by the compiler.
- **Whole-value moves only.** Owned values move as a unit. Moving an Owned field
  *out of* a struct/array (a partial move) is not allowed; read Copy fields
  freely, move the whole aggregate.
- **Branch-consistent moves (no runtime drop flags).** Within a scope, a binding
  must have the same moved/live status on all paths that rejoin. Moving a binding
  in one `if` arm but not another is a compile error, not a runtime drop flag.
  This keeps drops fully compile-time and adds **zero** runtime overhead, at the
  cost of some flexibility — a deliberate trade chosen over Rust-style drop flags.
- **No moves out of a loop binding.** A loop body may not move an Owned binding
  declared outside the loop (it would be moved on the next iteration). Owned
  values created inside the loop are dropped at each iteration's end.
- **Reading without consuming** is done with the `isi(k)` builtin (returns a copy
  of a `kotak`'s contents; requires the contents to be Copy). This is the
  borrow-read path while first-class references are pending.
- **References are Phase 9.** `&`/`&ubah` borrows with aliasing rules are a large
  subsystem; the design is written in `docs/MEMORY_MODEL.md`, and the
  implementation is deferred to Phase 9 exactly as the roadmap lists it. The
  Phase 8 model is coherent without them: owned values move, are read via `isi`,
  and are handed back by returning them.
- **FFI boundary.** Owned types may not appear in `ekstern` signatures or be
  passed to extern functions; the C ABI has no knowledge of Lime ownership, so
  crossing the boundary with an owned value is rejected at compile time.
- **Generics.** Ownership is enforced per monomorphized instance, where every
  type parameter is concrete and its Copy/Owned category is known. A generic body
  is thus checked for ownership violations once per instantiation.

## Implementation order (incremental, each step keeps tests green)

1. Types + classification (`Type::Kotak`, `is_copy`).
2. Syntax: `kotak<T>` type, `kotak(x)` / `isi(k)` builtins.
3. Ownership analysis in sema (moves, use-after-move, drop obligations).
4. IR + codegen for `kotak` allocation, `isi`, and drops (malloc/free).
5. Tests (valid + invalid), examples, docs.
