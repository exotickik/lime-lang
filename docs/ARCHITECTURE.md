# Lime Compiler Architecture

## Pipeline

```
Lime source (.lime)
      │  lime_lexer
      ▼
Token stream ──────────────► lime_diagnostics (shared across all stages)
      │  lime_parser
      ▼
AST (lime_ast)
      │  lime_sema: name resolution → type checking → control-flow checks
      ▼
Typed AST + symbol tables (lime_sema, lime_types)
      │  lime_ir: lowering
      ▼
Lime IR  (SSA-form, target-independent, Lime-owned)
      │  lime_ir::opt (constant folding, DCE, ...)
      ▼
Optimized Lime IR
      │  lime_codegen: emit textual LLVM IR (.ll)
      ▼
LLVM IR  ──llc──►  object file (.o)  ──cc/lld──►  native executable
```

`llc` and the system linker (`cc`, `clang`, or `lld`) are invoked as external
processes. **LLVM IR is not C/C++**; no stage ever emits or consumes C or C++
source. Using libc through the platform ABI (e.g. `printf`) is FFI, permitted by
the spec.

## Crate layout (Cargo workspace)

```
lime-lang/
├── Cargo.toml                 # [workspace]
├── crates/
│   ├── lime_span/             # SourceMap, FileId, Span, line/col resolution
│   ├── lime_diagnostics/      # Diagnostic, DiagnosticEngine, renderer, codes
│   ├── lime_lexer/            # Token, TokenKind, Lexer
│   ├── lime_ast/             # AST node definitions
│   ├── lime_parser/          # recursive-descent + Pratt expression parser
│   ├── lime_types/           # Type, TypeCtx, primitive registry
│   ├── lime_sema/            # scopes, symbols, resolver, type checker, cfg check
│   ├── lime_ir/              # Lime IR, lowering, optimizer passes
│   ├── lime_codegen/         # Lime IR -> LLVM IR text; toolchain invocation
│   ├── lime_fmt/             # deterministic formatter (AST -> source)
│   └── lime_driver/          # Session, orchestration of all stages
├── cli/                       # `lime` binary (argument parsing, subcommands)
├── stdlib/                    # Lime standard library sources (.lime)
├── examples/                  # sample Lime programs
└── tests/                     # integration tests driving the CLI
```

Each crate is independently testable. `lime_driver` owns the `Session` (source
map + diagnostic engine + options) threaded through every stage.

## Diagnostics subsystem

`lime_diagnostics` is a reusable engine independent of any single stage.

- `Diagnostic { severity, code, message, spans: Vec<Labeled Span>, notes }`
- `Severity`: `Error | Warning | Note | Help`.
- Stable codes: `E0xxx` errors, `W0xxx` warnings. Codes are documented in
  `crates/lime_diagnostics/src/codes.rs`.
- The renderer prints `file:line:col: severity[CODE]: message`, then the source
  line with a caret underline of the primary span, then notes/help.
- The engine accumulates diagnostics; a stage may emit many before aborting, so
  the user sees more than one error per run where safe.

Example output:
```
main.lime:12:15: error[E0201]: variabel 'umur' belum didefinisikan
   12 | tampilkan(umur)
      |           ^^^^ tidak ditemukan pada scope ini
```

## Module & name resolution

- A file is a module: a library file begins with `modul <Nama>`, the entry file
  with `program <Nama>` (and holds `utama`). `impor <Nama>` resolves the sibling
  file `<Nama>.lime`.
- The driver loads the entry file and every transitively `impor`ed file, sharing
  one AST node-id counter so ids stay unique, and merges all items into a single
  `Program`. A `loaded` set makes the load cycle-safe; a missing module file is a
  diagnostic. Each file keeps its own `SourceMap` id for diagnostics, and the
  driver records, per module, its name and its import set.
- `lime_sema` receives this `ModuleInfo`. During collection it records each
  top-level symbol's owning module (by span file id) and its `publik` flag. While
  checking a body it sets the current module, and the name/type resolvers reject a
  reference to a symbol the current module cannot reach — not its own and not a
  `publik` item of an `impor`ed module — as `E02C0`. Name resolution itself stays
  flat, so type checking, ownership, generics and traits work unchanged across
  module boundaries.
- Visibility: items are module-private unless prefixed with `publik`. `gunakan`
  is reserved for future standard-library modules.

## Target abstraction

`lime_codegen::target::Target` captures: pointer width, data layout string,
target triple, integer/float ABI, and the linker driver + flags. Bootstrap
targets:

| Target | Triple | Linker | Status |
|---|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | `cc` / `lld` | implemented & validated |
| Windows x64 | `x86_64-pc-windows-gnu` | `lld-link` | structurally wired, unverified here |

No Windows-specific assumptions leak outside `target.rs`; the rest of the
compiler is target-agnostic.
