# Phase 30 (Generic structs) — Design Assessment

Baseline: `main` after owned `Hasil` payloads (Phase 29). Next in the self-hosting
order: real generic user-defined aggregate types. This phase delivers generic
structs.

## Goal

`tipe Nama<T, U> { … }` — real generic structs that integrate with type checking,
monomorphization, field access, ownership, references, nesting, and native
codegen, with no runtime cost.

## Approach — monomorphize to concrete structs

A generic struct is stored as a **template** (its `StructDecl` with `type_params`),
not as a `StructId`. Each distinct application `Nama<A, B>` is **monomorphized** on
demand into an ordinary concrete `StructInfo`/`StructId` with the type parameters
substituted in the field types. Because the result is a normal concrete struct,
**all existing struct IR and codegen are reused unchanged** — the feature is
entirely front-end.

- `resolve_type` on `Applied { name, args }` where `name` is a generic struct
  template checks arity, resolves the args, and calls `monomorphize_struct`,
  returning `Type::Struct(id)`.
- `monomorphize_struct(tidx, args)` caches by `(template, args)`, reserves the
  `StructId` with a mangled name **before** resolving fields (so self-referential
  and mutually-referential instantiations terminate), substitutes the args into
  each field `TypeExpr`, and resolves them. The mangled instance name is also
  registered in `struct_ids`, so a substituted field type that refers to another
  monomorphized struct round-trips cleanly through name resolution — which is what
  makes nesting (`Pasangan<Titik<angka>, angka>`) work.

## Type arguments at a struct literal

A struct literal carries no explicit type arguments in the grammar, so the
concrete instantiation is found by:
1. the **expected type** — if it is a `Struct` that is an instantiation of this
   template (tracked in `struct_instance_of`), it is used directly; else
2. **inference** — each field value is checked and unified against the template's
   declared field type to solve the type parameters, then monomorphized.

Expected-type resolution covers the common cases (`nilai p: Nama<…> = Nama { … }`,
return position, call arguments); inference covers the rest.

## Ownership

No new ownership code: a monomorphized struct has concrete field types, so the
existing `is_copy`/drop/`move` machinery classifies it correctly — a generic
struct with an Owned field is Owned and moves (use-after-move enforced), and its
fields drop recursively. Verified valgrind-clean.

## Tests

- `cli/tests/generic_structs.rs`: two type params, inference from return position,
  multiple instantiations, nesting, generic struct inside `daftar`, owned-field
  struct (move) at runtime, use-after-move rejection, wrong arity, unknown generic
  type, field type mismatch.
- `exotickik_sema`: generic struct ok, wrong arity, owned-field move.
- `examples/pasangan.exo`: `Pasangan<T, U>`, `Titik<T>`, a swap function, and a
  nested `Pasangan<Titik<angka>, angka>`.

All prior tests pass; 0 warnings; repo passes `exotickik fmt --check`.

## Honest limitations

- No explicit type arguments on a struct literal (`Nama<…> { … }`); args come from
  the expected type or field-value inference. A field value that itself needs a
  type hint to infer (e.g. a bare `daftar()`) requires the expected type.
- A generic struct whose field is **another generic struct parameterized by this
  struct's own type parameter** is uncommon and resolved only when that nested
  type becomes concrete via substitution; a template-level validation of such a
  field is limited.
- No trait bounds enforced on struct type parameters yet (bounds parse).

## Next

Generic enums, then enum associated data (payload-carrying variants) with `cocok`
binding — which also unblocks owned-`Hasil` inspection — then porting compiler
front-end components into Exotickik.
