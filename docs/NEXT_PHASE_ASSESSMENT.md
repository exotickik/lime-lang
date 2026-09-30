# Phase 13 (Modules) — Design Assessment

Baseline: `main` at the Error-Handling checkpoint — 215 tests, 0 warnings, all
examples run, formatter idempotent.

## Current state

- The driver compiles a **single** `.lime` file: `frontend(path)` reads one file,
  lexes, parses, and analyzes it.
- `impor <Name>` and `gunakan <Name>` parse into `Item::Import` / `Item::Use` and
  are otherwise ignored.
- `publik` is parsed on functions, structs, enums, and struct fields but is not
  enforced across any boundary (there is only one file).

## Goal

Real multi-file compilation: a program can be split across `.lime` files, one file
can `impor` another, and `publik` controls what crosses the module boundary. The
result compiles and runs as one native executable.

## Model (v1)

- **A module is a file.** A library file begins with `modul <Nama>`; the entry
  file begins with `program <Nama>` and holds `utama`. An imported file may not
  contain `utama`.
- **`impor <Nama>`** loads `<Nama>.lime` from the importing file's directory,
  transitively. The import graph is walked once; a cycle is a diagnostic, and a
  missing file is a diagnostic.
- **Visibility.** Within its own module a name is always accessible. A name from
  another module is accessible only if that module is `impor`ed by the referrer
  **and** the item is `publik`. This applies to functions, structs, enums,
  traits, and (for field access) struct fields. A violation is `E0270`-class
  diagnostic `E02C0`.
- **Flat namespace (documented limitation).** All modules currently share one
  global name space: two modules may not define the same top-level name (that is
  the existing redefinition error). True per-module namespacing with qualified
  names (`Matematika.tambah`) is the documented next step; this phase delivers
  separate files, imports, and enforced visibility without it.

## Implementation

Modules are resolved by a **front-end merge pass** that produces a single
`Program`, so the existing sema / IR / codegen run unchanged:

1. **Driver** loads the entry file and every transitively `impor`ed file (same
   directory), each into its own `SourceMap` file id, sharing one AST node-id
   counter so ids stay unique. It concatenates all items into one `Program` and
   records module metadata: the module name per file id, the import set per
   module, and — from the merged items — which module defines each top-level name
   and whether it is `publik`.
2. **Parser** accepts `modul <Nama>` as a header alongside `program <Nama>`
   (`Program.is_module`), and rejects `utama` in a module during later checking.
3. **Sema** takes the module metadata. During collection it records each global
   symbol's owning module and visibility. While checking a function/`utama`
   body it sets the current module from the item's file id, and the global name
   resolver (`lookup`) and type resolver reject a reference to a symbol that the
   current module cannot access (`E02C0`) — locals, builtins and primitive types
   are unaffected. Name resolution itself is unchanged (flat), so type checking,
   ownership, generics and traits keep working across module boundaries.

## Validation

Multi-file Lime programs (an entry `program` plus one or more `modul` files) must
compile with `lime build`/`run`, exercise cross-module functions/structs/traits,
and reject access to private items and un-imported modules. A runnable multi-file
example ships under `examples/`.

## Non-goals this phase (documented, not faked)

- Per-module namespaces and qualified names (`Modul.item`); same-name items in
  different modules.
- Package-manager / dependency resolution (a later roadmap milestone).
- Standard-library modules via `gunakan` (kept reserved).
