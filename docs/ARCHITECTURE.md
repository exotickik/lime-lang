# Exotickik Compiler Architecture

## Pipeline

```
Exotickik source (.exo)
      │  exotickik_lexer
      ▼
Token stream ──────────────► exotickik_diagnostics (shared across all stages)
      │  exotickik_parser
      ▼
AST (exotickik_ast)
      │  exotickik_sema: name resolution → type checking → control-flow checks
      ▼
Typed AST + symbol tables (exotickik_sema, exotickik_types)
      │  exotickik_ir: lowering
      ▼
Exotickik IR  (SSA-form, target-independent, Exotickik-owned)
      │  exotickik_ir::opt  (constant fold + propagation, constant-branch fold, DCE;
      │                 fixpoint, gated by opt level — skipped at -O0)
      ▼
Optimized Exotickik IR
      │  exotickik_codegen: emit textual LLVM IR (.ll)
      ▼
LLVM IR  ──llc -O{0..3}──►  object file (.o)  ──cc/lld──►  native executable
```

The Exotickik IR passes normalize and shrink the IR (fold constants, simplify
constant branches, drop dead pure instructions); the heavy optimization
(inlining, loop transforms, redundant load/store elimination) is delegated to
LLVM at the selected `-O` level rather than reimplemented. `-O0` skips the Exotickik
passes and asks `llc` for no optimization; `-O1..-O3` run the Exotickik passes and the
matching `llc` level.

`llc` and the system linker (`cc`, `clang`, or `lld`) are invoked as external
processes. **LLVM IR is not C/C++**; no stage ever emits or consumes C or C++
source. Using libc through the platform ABI (e.g. `printf`) is FFI, permitted by
the spec.

## Crate layout (Cargo workspace)

```
exotickik-lang/
├── Cargo.toml                 # [workspace]
├── crates/
│   ├── exotickik_span/             # SourceMap, FileId, Span, line/col resolution
│   ├── exotickik_diagnostics/      # Diagnostic, DiagnosticEngine, renderer, codes
│   ├── exotickik_lexer/            # Token, TokenKind, Lexer
│   ├── exotickik_ast/             # AST node definitions
│   ├── exotickik_parser/          # recursive-descent + Pratt expression parser
│   ├── exotickik_types/           # Type, TypeCtx, primitive registry
│   ├── exotickik_sema/            # scopes, symbols, resolver, type checker, cfg check
│   ├── exotickik_ir/              # Exotickik IR, lowering, optimizer passes
│   ├── exotickik_codegen/         # Exotickik IR -> LLVM IR text; toolchain invocation
│   ├── exotickik_fmt/             # deterministic formatter (AST -> source)
│   └── exotickik_driver/          # Session, orchestration of all stages
├── cli/                       # `exotickik` binary (argument parsing, subcommands)
├── stdlib/                    # Exotickik standard library sources (.exo)
├── examples/                  # sample Exotickik programs
└── tests/                     # integration tests driving the CLI
```

Each crate is independently testable. `exotickik_driver` owns the `Session` (source
map + diagnostic engine + options) threaded through every stage.

## Diagnostics subsystem

`exotickik_diagnostics` is a reusable engine independent of any single stage.

- `Diagnostic { severity, code, message, spans: Vec<Labeled Span>, notes }`
- `Severity`: `Error | Warning | Note | Help`.
- Stable codes: `E0xxx` errors, `W0xxx` warnings. Codes are documented in
  `crates/exotickik_diagnostics/src/codes.rs`.
- The renderer prints `file:line:col: severity[CODE]: message`, then the source
  line with a caret underline of the primary span, then notes/help.
- The engine accumulates diagnostics; a stage may emit many before aborting, so
  the user sees more than one error per run where safe.

Example output:
```
main.exo:12:15: error[E0201]: variabel 'umur' belum didefinisikan
   12 | tampilkan(umur)
      |           ^^^^ tidak ditemukan pada scope ini
```

## Module & name resolution

- A file is a module: a library file begins with `modul <Nama>`, the entry file
  with `program <Nama>` (and holds `utama`). `impor <Nama>` resolves the sibling
  file `<Nama>.exo`.
- The driver loads the entry file and every transitively `impor`ed file, sharing
  one AST node-id counter so ids stay unique, and merges all items into a single
  `Program`. A `loaded` set makes the load cycle-safe; a missing module file is a
  diagnostic. Each file keeps its own `SourceMap` id for diagnostics, and the
  driver records, per module, its name and its import set.
- `exotickik_sema` receives this `ModuleInfo`. During collection it records each
  top-level symbol's owning module (by span file id) and its `publik` flag. While
  checking a body it sets the current module, and the name/type resolvers reject a
  reference to a symbol the current module cannot reach — not its own and not a
  `publik` item of an `impor`ed module — as `E02C0`. Name resolution itself stays
  flat, so type checking, ownership, generics and traits work unchanged across
  module boundaries.
- Visibility: items are module-private unless prefixed with `publik`. `gunakan`
  is reserved for future standard-library modules.

## Target abstraction

`exotickik_codegen::target::Target` captures: pointer width, data layout string,
target triple, integer/float ABI, and the linker driver + flags. Bootstrap
targets:

| Target | Triple | Linker | Status |
|---|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | `cc` / `lld` | implemented & validated |
| Windows x64 | `x86_64-pc-windows-gnu` | `lld-link` | structurally wired, unverified here |

No Windows-specific assumptions leak outside `target.rs`; the rest of the
compiler is target-agnostic.
