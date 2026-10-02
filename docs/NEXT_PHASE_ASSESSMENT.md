# Phases 31–32 (Enum associated data + generic enums) — Design Assessment

Baseline: `main` after generic structs (Phase 30). Next in the self-hosting
order: generic enums, then enum associated data. A payload-less generic enum has
a phantom type parameter (its `T` is unused until a variant carries it), so the
two are naturally one feature and associated data is the valuable half. This
phase delivers **enum variants with associated data** (non-generic sum types)
plus `cocok` payload binding; generic enums follow once there is a payload for
`T` to parameterize.

## Goal

```exotickik
enum Token { Angka(angka) Operator(char) Akhir }

cocok t {
    Angka(n) => { ... }        // bind the payload
    Operator(c) => { ... }
    Akhir => { ... }
}
```

A variant may carry positional payload values; it is constructed by calling it
(`Token.Angka(10)`) and destructured by binding in a `cocok` arm — integrated
with type checking, exhaustiveness, ownership, native construction and matching,
with no runtime indirection.

## Representation — tagged aggregate, reusing struct codegen

A payload-carrying enum lowers to a flattened tagged aggregate:
`{ i32 tag, <every variant's payload fields, in declaration order> }`. Only the
active variant's slots are live; the rest are `undef`. This is exactly the scheme
`Hasil<T, R>` already uses (`{ i32, T, R }`), generalized to N variants with
positional payload lists.

- In LLVM each enum becomes a named type `%enum.N` — `%enum.N = type i32` for a
  tag-only enum (identical lowering to before), or `%enum.N = type { i32, … }`
  for a payload-carrying one. `ty_str(Enum)` emits `%enum.N` with no context
  table; the layout is needed only where the aggregate is built (`MakeStruct`),
  which is handed the enum table alongside the struct table. Named enum types are
  emitted before struct types so struct fields can reference them.
- Construction fills the tag and the active variant's fields (`MakeStruct`),
  leaving the rest `undef` (via `undef_of`, as `Hasil` does). A tag-only enum
  still lowers to a scalar `i32` constant.
- A `cocok` arm reads the tag field (`ExtractField` 0), compares it, and for each
  binding extracts the active variant's payload field (`ExtractField` at the
  variant's flattened offset) into a fresh slot bound to the arm-scoped local —
  mirroring how `untuk … pada` binds its loop variable.

Because a payload enum is an ordinary LLVM aggregate, construction, matching,
storage in `daftar`/structs, pass/return by value, and variant-name printing all
reuse the existing aggregate machinery. A `daftar<Token>` (a real token stream)
works and is valgrind-clean.

## Front end

- **AST/parser**: `EnumDecl` gains `type_params` (parsed now for the generic-enum
  step) and variants become `EnumVariant { name, payload: Vec<TypeExpr> }`;
  `MatchArm` gains `bindings` + `binding_ids`. The parser reads `Var(T1, T2)` in
  a declaration and `Var(a, b)` in a `cocok` arm.
- **Sema**: enum names are collected first, then payloads are resolved in a
  second pass (after structs, so a payload may name a struct). A variant call is
  recognized as construction (arity + per-field type checks). `cocok` validates
  binder arity, scopes binding locals with their payload types, and keeps
  exhaustiveness. `is_copy(Enum)` follows the payloads.

## Ownership

Payloads are restricted to **Copy** types this phase (integers, `char`,
`boolean`, `teks`, another enum, a Copy struct/array) — which covers token kinds,
literals, and arena indices, i.e. the self-hosting AST use. An owned payload
type (`daftar`, `kotak`, `peta`) is rejected at the declaration (`E0902`). Copy
payloads need no drop or move machinery, so matching copies the payload out and
leaves the scrutinee intact. A payload-carrying enum also drops `==`/`!=`
(comparing aggregates with `undef` padding is meaningless — use `cocok`); a
tag-only enum keeps equality.

## Tests

- `cli/tests/enum_payload.rs`: single/multiple payloads, mixed payload/plain
  variants, payload enum in `daftar` (token stream), `teks` payload, nested enum
  payload, enum as struct field, variant-name printing, round-trip through
  functions, payload-ignoring match, and the error cases (construction arity,
  binder-count mismatch, binding a plain variant, bare payload variant, owned
  payload, equality on a payload enum, argument type mismatch).
- `exotickik_sema`: construct+match ok, wrong arity, owned payload rejected,
  equality rejected.
- `examples/ekspresi.exo`: a register-machine interpreter whose instruction type
  is a payload-carrying `enum` held in a `daftar`.

All prior tests pass (374 total); 0 warnings; repo passes `exotickik fmt --check`;
valgrind-clean including the `daftar<Token>` program.

## Honest limitations

- **Copy payloads only.** Owned payload types are rejected. Owned payloads need
  tag-conditional drop (the `Hasil` machinery generalizes) plus move-out binding
  that consumes the scrutinee; deferred to keep this phase sound and complete.
- A payload enum is not comparable with `==`/`!=`.

## Generic enums (Phase 32, shipped on top of this)

`enum Pilihan<T> { Ada(T) Kosong }` is now monomorphized exactly like generic
structs: the template is stored, and each application `Pilihan<angka>` becomes a
concrete `EnumInfo`/`EnumId` with `T` substituted into the variant payloads (the
mangled instance name is registered in `enum_ids` so it round-trips through name
resolution). The concrete enum is an ordinary one, so all the associated-data IR
and codegen above are reused unchanged. The instantiation is taken from the
expected type when it is a matching instance, otherwise inferred by unifying a
variant's payload arguments against its declared payload (`Pilihan.Ada(5)` solves
`T = angka`); a payload-less variant with no expected type cannot be inferred and
is a `E0270` error. Tests in `cli/tests/generic_enums.rs` and `exotickik_sema`,
plus `examples/pilihan.exo`.

## Next

Owned enum payloads (construction + tag-conditional drop + move-binding), which
also unblocks owned-`Hasil` inspection via `cocok`. Then begin porting the
bootstrap front end's data structures to Exotickik sum types.
