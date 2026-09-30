# Phase 11 (Traits) — Design Assessment

Baseline before this phase: `cargo build --workspace` and `cargo test --workspace`
green (175 tests, 0 warnings). Phases 8–9 (ownership, references) are in place and
must not regress. References, generics with monomorphization, structs and enums
are all available and are the foundations traits build on.

## Goal

A real, first-class **trait** system with **static dispatch** (monomorphization).
No dynamic dispatch / `dyn` in this phase — static dispatch is preferred and the
architecture does not yet need vtables.

## Why this fits the existing architecture

- Generic functions are already monomorphized: each `(template, type-args)` pair
  becomes a fresh `lime_ast::Function` (name-mangled), whose signature and body
  are **re-checked** with the concrete types substituted, then lowered like any
  other function. Trait method calls inside a generic body therefore resolve to a
  concrete impl automatically once the type parameter is concrete — this *is*
  static dispatch, with no new lowering concept.
- Method-call syntax needs no new grammar: `x.m(a)` already parses as
  `Call { callee: Field { base: x, field: m }, args: [a] }`. The checker treats a
  call whose callee is a field access naming a method as a method call.
- An impl method is just a function. Each `implementasi` method lowers to an
  ordinary mangled top-level function (e.g. `Titik$luas`); a method call becomes a
  direct call to it with the receiver as the first argument. All existing
  function checking, IR lowering, and codegen are reused unchanged.

## Surface syntax (Indonesian, consistent with Lime)

```lime
sifat Luas {
    fungsi luas(diri: &Diri) -> angka
}

implementasi Luas untuk Persegi {
    fungsi luas(diri: &Diri) -> angka { kembalikan diri.sisi * diri.sisi }
}

implementasi Persegi {
    fungsi perbesar(diri: &ubah Diri, k: angka) { diri.sisi = diri.sisi * k }
}

fungsi total_luas<T: Luas>(bentuk: &T) -> angka {
    kembalikan bentuk.luas()
}
```

- `sifat Nama { ... }` declares a trait; its body holds **method signatures only**
  (no bodies — default methods are a later extension, not faked here).
- `implementasi Trait untuk Tipe { ... }` provides the trait's methods for a type;
  `implementasi Tipe { ... }` provides inherent methods (no trait).
- `Diri` is the Self type; `&Diri` / `&ubah Diri` / `Diri` choose the receiver
  mode (shared borrow / mutable borrow / by-value). The receiver parameter name is
  conventional (`diri`) but not special — only its type position matters.
- Type-parameter bounds: `<T: Trait>` and `<T: A + B>`.

## Representation

- **AST.** `TypeParam { name, bounds: Vec<Ident> }` replaces the bare `Ident` in
  `type_params`. New items `Item::Trait(TraitDecl)` and `Item::Impl(ImplBlock)`.
  `TraitDecl` holds `MethodSig`s (name, params, ret — no body). `ImplBlock` holds
  an optional trait name, a target type name, and method `Function`s.
- **Self type.** `Diri` resolves to `Type::Param("Diri")` inside a trait or impl,
  reusing the generic substitution engine. When checking or lowering an impl for
  type `S`, `Param("Diri")` is substituted with `S`.
- **Sema.** `TraitInfo { name, methods: [ (name, param-types, ret) ] }` with method
  types resolved using the `Diri = Param("Diri")` convention. `ImplInfo
  { trait, self_type, methods: name -> fn-index }`. Each impl method is registered
  as a mangled `FnSig` and its body added to the check/lower pipeline (self-type
  substituted), exactly like a monomorphized generic.

## Method resolution & static dispatch

- **Concrete receiver.** For `x.m(args)` with `typeof(x)` concrete (auto-dereferencing
  a reference receiver), look up an impl method `m` for that type. Exactly one match
  is required; none → `E02A2`, ambiguous (two traits) → `E02A3`. The method's first
  parameter type selects the receiver mode: `&Diri` borrows `x`, `&ubah Diri`
  mutably borrows it, `Diri` moves it — so a method call creates the same loans /
  moves as `ambil` / `ambil_ubah` / a by-value pass, and the borrow checker treats
  it identically.
- **Bounded generic receiver.** Inside a generic body, `x.m()` for `x : T` with
  `T: Trait` type-checks against the trait's signature (with `Diri = T`). No
  concrete call is emitted for the template — the template body is only checked
  through its instantiations, where `T` is concrete and resolution finds the real
  impl.
- **Bound enforcement.** At a call to `f<T: Trait>(..)`, once `T` is inferred, the
  concrete type must implement every bound trait (`E02A4`) — this gives a clean
  error at the call rather than deep inside the instantiated body, and guarantees
  every method call in the body resolves.

## Diagnostics (E02A0–E02A6)

| Code | Meaning |
|---|---|
| E02A0 | trait method with a duplicate name |
| E02A1 | impl is missing a trait method / has an unknown or mis-typed one |
| E02A2 | method not found for the receiver type |
| E02A3 | ambiguous method (defined by more than one implemented trait) |
| E02A4 | type does not implement a required trait bound |
| E02A5 | duplicate implementation of a trait for a type |
| E02A6 | unknown trait or impl target type |

## Ownership / references / generics / FFI interaction

- **Ownership & references.** The receiver mode reuses the existing loan and move
  machinery — no new borrow rules. A `&ubah Diri` method needs a mutable place,
  and no other borrow may be live, exactly as `ambil_ubah`.
- **Generics.** Traits compose with monomorphization: bounded generic functions
  are checked per instance, so method calls resolve statically with zero overhead.
- **FFI.** Traits are a compile-time resolution mechanism; they add no runtime
  representation and never cross the FFI boundary. Extern functions are unaffected.

## Non-goals this phase (documented, not faked)

- No dynamic dispatch / trait objects (`dyn`).
- No default method bodies in traits.
- No associated types or associated constants.
- No blanket impls or generic impls (`implementasi<T> Trait untuk kotak<T>`).

These are natural later extensions; the phase ships a complete, statically
dispatched trait system without them.
