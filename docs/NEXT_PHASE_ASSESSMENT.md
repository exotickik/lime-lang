# Phase 21 (FFI expansion) — Design Assessment

Baseline: `main` after the benchmark runner (Phase 20). Sixth self-hosting
prerequisite item: widen the FFI surface — callbacks and external pointer types —
without weakening Lime's memory safety.

## Goal

Let Lime bind more real C APIs: pass Lime functions to C as callbacks, and carry
opaque C handles (`FILE*`, allocator handles, library contexts) in and out of
`ekstern` functions — while keeping the rule that Lime never reads or writes
foreign memory unchecked.

## Callbacks — already supported, now verified

Passing a Lime function to a native function that expects a function pointer
already works: `ekstern` parameters accept a function type (`fungsi(A) -> R`,
resolved to `Type::Fn`), a function name lowers to a bare function pointer
(`Operand::Func`), and the C side calls it over the ABI. This phase adds
coverage: a libc `atexit` callback test, a runnable example, and documentation.
No code change was needed — the first-class-function work (Phase, `fungsi` values)
and the existing extern lowering already compose correctly across the boundary.

## Opaque pointer type (`pointer`) — the new capability

Lime forbids references across FFI (`E0298`) because a Lime reference carries
borrow/lifetime guarantees C cannot honor. But many C APIs traffic in opaque
handles that Lime only needs to *hold and pass back*, never dereference. The file
I/O work (Phase 18) smuggled `FILE*` through as `i64`, which is untyped and
error-prone.

`pointer` is a first-class primitive for exactly this:

- **Opaque.** It cannot be dereferenced (`*p` → `E0292`), cast to or from an
  integer (`E02F0`), or printed (`E0211`). There is no operation that reads or
  writes through it, so it introduces no unsafety — it is just a token.
- **Copy and FFI-safe.** It is Copy (passed/returned/stored freely, never
  dropped) and, being neither a reference nor Owned, crosses the FFI boundary.
- **Constructed only at the boundary or as null.** A `pointer` value comes from
  an `ekstern` return or from the `kosong()` builtin (the null pointer); Lime
  code cannot fabricate an arbitrary address.
- **Comparable.** `==` / `!=` are allowed on `pointer` (identity / null checks),
  which is what makes `fopen` → null-check → `fclose` expressible.

This lets users bind C handle APIs directly and type-safely:

```lime
ekstern fungsi fopen(nama: teks, mode: teks) -> pointer
ekstern fungsi fclose(f: pointer) -> i32
```

## Implementation

A new `Type::Ptr` variant, threaded through the usual sites:

- `lime_types`: `Type::Ptr`, `from_builtin_name("pointer")`, `display` → `pointer`.
  `resolve_type` picks it up automatically via `from_builtin_name`.
- `lime_sema`: `Builtin::Kosong` registered as `kosong` (no args → `Type::Ptr`);
  `==`/`!=` extended to accept `Ptr`; `is_copy`/`owned` default make `Ptr` Copy
  and non-Owned, so the existing FFI check (reject Ref/Owned) admits it. Deref,
  cast, and print checks already reject it through their existing type guards.
- `lime_ir`: `Builtin::Kosong` lowers to `Operand::Null`; comparisons lower to a
  `Cmp` over the operand type, and `pointer` operands are already typed `ptr`.
- `lime_codegen`: `ty_str(Type::Ptr)` → `ptr`; `Operand::Null` already emits
  `null : ptr`, and `icmp eq/ne ptr` is valid LLVM.

## Tests

- `cli/tests/ffi.rs`: a libc `atexit` callback, `fopen`/`fclose` with a
  `== kosong()` null check (present vs missing file), `*pointer` rejected, int→
  `pointer` cast rejected, and a confirmation that references are still rejected
  across FFI.
- `lime_sema`: `opaque_pointer_ffi_ok`, `pointer_cannot_be_dereferenced`,
  `pointer_cast_rejected`, `pointer_is_copy_not_owned`.
- `examples/pointer_ekstern.lime`: a callback (`atexit`) plus `pointer`-based
  file-open checks.

## Honest limitations

- `pointer` is opaque only — there is intentionally no way to read/write through
  it from Lime. Binding an API that needs Lime to pass a buffer C writes into
  still has no safe path (references remain forbidden across FFI); a future
  owned-buffer handoff or an explicit `tak_aman` block could add one.
- No struct-pointer field access, pointer arithmetic, or typed pointers (`*T`).
- Callback ABI is whatever the platform C ABI is; no calling-convention
  attributes are exposed.
- Linking is still libc-only; custom `-l`/link flags await the package system.
