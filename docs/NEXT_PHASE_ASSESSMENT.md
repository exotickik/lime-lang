# Next-Phase Implementation Assessment

Baseline recorded before new work: `cargo build --workspace` and
`cargo test --workspace` both green (65 tests).

## What already exists

| Area | State |
|---|---|
| Lexer, AST, parser, diagnostics | complete for the current language subset |
| Semantic analysis + type checking | functions, bindings, expressions, control flow, calls |
| Struct **declarations** | parsed, `StructId` assigned, field types resolved |
| Struct **field access** (read) | type-checked in `lime_sema::check::check_field` |
| Struct **field mutation** legality | checked via `check_assignable` (root-local mutability) |
| Lime IR + optimizer | alloca/load/store CFG, constant folding, DCE |
| LLVM backend | scalar types, functions, control flow, `printf` via FFI |
| CLI, formatter, docs, tests | present and green |

## What is missing for the data model (Phase 4)

Structs are only half-wired: there is **no way to construct a struct value**,
and the backend cannot represent aggregates.

Concretely, Phase 4 requires:

1. **AST**: a struct-literal expression (`Nama { field: expr, ... }`).
2. **Parser**: parse struct literals, with a struct-literal restriction inside
   `jika` / `selama` / `untuk` conditions to avoid the `Name {` / block
   ambiguity (parenthesize to override).
3. **Semantics**: type-check struct literals (all fields present exactly once,
   correct types, no unknown fields); field access and mutation already exist.
4. **Lime IR**: aggregate operations — `MakeStruct`, `ExtractField`,
   `InsertField` — plus a module-level struct layout table. Struct locals reuse
   the existing slot load/store since a slot already carries its element type.
5. **Codegen**: emit named LLVM struct types (`%struct.N`), lower the aggregate
   ops to `insertvalue` / `extractvalue`, and pass/return structs by value.
6. **Tests + examples + docs**: real Lime programs that construct, read, mutate,
   nest, pass, and return structs, compiled and executed.

## Representation decision

Structs are compiled to real LLVM aggregate types (`{ ... }`), not maps or
dynamic objects. Construction uses `insertvalue` from `undef`; reads use
`extractvalue`; whole-struct locals load/store the aggregate; mutation of
`x.a.b` loads the root aggregate, applies a nested `insertvalue`, and stores it
back. `llc -O2` (SROA/mem2reg) promotes these to registers, so simple struct
code has zero heap cost and no hidden allocation — consistent with the memory
model.

## Ordering of subsequent phases

Structs (4) → enums (5) → typed arrays/collections (6) → string model (7) →
memory model rules (8) onward, per the roadmap. Each lands fully (implementation,
integration, tests, real programs, docs) before the next begins.
