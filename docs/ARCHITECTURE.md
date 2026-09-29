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

## Module & name resolution (design)

- A `program` is one compilation unit. `gunakan <std module>` pulls a standard
  library module; `impor <name>` resolves a sibling `.lime` file.
- Resolution builds a module graph; a DFS with a "visiting" color set detects
  circular imports and reports `E0110`.
- Visibility: items are module-private unless prefixed with `publik`.

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
