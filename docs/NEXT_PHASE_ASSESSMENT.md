# Phase 9 (References) — Design Assessment

Baseline before this phase: `cargo build --workspace` and `cargo test --workspace`
green (141 tests, 0 warnings). Phase 8 ownership (`kotak<T>`, moves, drops) is in
place and must not regress.

## Current state relevant to references

- Every local lives in a stack slot (`alloca`, tracked by `slot_of_local`). The
  slot pointer is already a real `ptr` in the generated LLVM. **A reference to a
  local is therefore just the address of its slot** — no new runtime concept is
  needed, and the IR already has `LoadPtr` / `StorePtr` for pointer access.
- `Type::Kotak(T)` is the only Owned type; ownership is classified by `is_copy`
  in `lime_sema` and enforced flow-sensitively in `ownership.rs`.
- Borrowing already exists in a limited form: `isi(k)` reads a `kotak` without
  consuming it (a builtin marked "borrow-only" in `ownership.rs::visit_call`).

## Reference model (Lime's own design)

Lime references are **temporary, non-escaping borrows** checked entirely at
compile time. There is no reference counting, no runtime borrow state, and no GC.

### Types
- `&T` — an immutable reference (a **pinjam**, "borrow"): read the referent.
- `&ubah T` — a mutable reference: read and write the referent.

### Creating references
Two builtins, consistent with `kotak`/`isi`:
- `ambil(x)` → `&T` (take an immutable reference to local `x`).
- `ambil_ubah(x)` → `&ubah T` (take a mutable reference; `x` must be `ubah`).

`ambil`/`ambil_ubah` borrow a **whole local** and do **not** move it, including
when `x` is a `kotak<T>` (the box is borrowed, not duplicated).

### Using references
- `*r` — dereference: read the referent (requires a Copy referent), yielding a
  value. Lowers to a real load through the pointer.
- `*r = v` — dereference-assign through a `&ubah` reference: a real store through
  the pointer. `(*r).field` (with parentheses) accesses a field of a referenced
  struct.
- `isi(r)` accepts `&kotak<T>` / `&ubah kotak<T>` as well as `kotak<T>`, so a
  borrowed box can be read.

### Aliasing rules (checked in `ownership.rs`)
While a local `x` is borrowed (a **loan** is active):
- a `&ubah` loan is **exclusive**: no other access to `x` (read, write, move, or
  another borrow) may occur while it is live;
- `&` loans are **shared**: any number may coexist and `x` may be read, but `x`
  may not be mutated, moved, or `&ubah`-borrowed while one is live.

Loan lifetime is **lexical and conservative**: a loan bound to `nilai r = ...`
lives until the end of `r`'s block; a loan created as a call argument lives for
that statement. This is sound (it over-approximates a value's borrow span) and
simple; it may reject some programs a non-lexical analysis would accept, which is
a documented trade, not a soundness hole.

### No dangling — the non-escaping rule
A reference must never outlive its referent. Lime guarantees this structurally
rather than with lifetime parameters: **references may not escape**. Concretely,
the compiler rejects
- returning a reference from a function,
- storing a reference in a struct field, array element, or `kotak`,
- reference types crossing the FFI boundary.

References may only flow *downward* (into function calls) and be dereferenced.
Because a non-escaping reference cannot leave the scope in which its referent is
live, it can never dangle. Reference-returning functions (and thus generic
`ambil_maksimum<T>(a: &T, b: &T) -> &T`) need lifetime inference and are
explicitly deferred; the compiler rejects them with a clear diagnostic rather
than accepting them unsoundly.

### Ownership interaction
- `ambil(x)` / `ambil_ubah(x)` borrow, they do not move `x`; `x` stays live.
- A borrowed local may not be moved while the loan is active (caught as a loan
  conflict), so a value is never freed while a reference to it exists.
- References are Copy values (copying a reference is fine); the loan stays tied
  to the original borrow's lexical region.

### Generics
`ambil`/`*`/`ambil_ubah` work when the referent type is a generic parameter `T`
(checked per monomorphized instance). Returning a reference from a generic
function is rejected like any reference return.

## Implementation plan (incremental, tests stay green each step)

1. **Types + syntax**: `Type::Ref { inner, mutable }`; parse `&T` / `&ubah T`
   type forms; single `&` lexer token; prefix `*` deref expression and
   `*r = v` assignment target; `ambil` / `ambil_ubah` builtins.
2. **Type checking**: reference types, deref, `ambil`/`ambil_ubah`, `isi` on
   references.
3. **IR + codegen**: `AddrOf { slot }` (slot address), reuse `LoadPtr`/`StorePtr`
   for deref read/write; `ty_str(Ref) = ptr`. Real native pointer ops.
4. **Borrow checking** in `ownership.rs`: loans, aliasing conflicts, escape
   rejection, FFI rejection, move-while-borrowed.
5. **Tests** (valid + invalid), **examples**, **docs**.

Nothing in the Copy/ownership paths changes, so existing structs, enums, arrays,
strings, generics, FFI, and ownership behavior are preserved.
