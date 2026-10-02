# Exotickik

Exotickik is an independent, statically-typed, natively-compiled, general-purpose
programming language with a clean, modern syntax whose keywords are Indonesian.

Exotickik is **not** a C/C++ wrapper, transpiler, or macro layer. It has its own
lexer, parser, AST, semantic analysis, type system, SSA-style intermediate
representation, optimizer, and native code generation. LLVM is used strictly as
a backend: Exotickik lowers its own IR to LLVM IR, which `llc` turns into real machine
code. No stage ever emits or consumes C or C++.

```exotickik
program Fibonacci

fungsi fib(n: angka) -> angka {
    jika n < 2 {
        kembalikan n
    }
    kembalikan fib(n - 1) + fib(n - 2)
}

utama {
    untuk i dari 0 sampai 10 {
        tampilkan(fib(i))
    }
}
```

```
$ exotickik run examples/fibonacci.exo
0
1
1
2
3
5
8
13
21
34
```

## Status

The bootstrap compiler is a complete **vertical slice**: it lexes, parses,
type-checks, lowers to a Exotickik-owned IR, optimizes, emits LLVM IR, and links a
**real native executable** that runs and produces correct output — for the
numeric, function, and control-flow core of the language.

Implemented and tested end-to-end:

- primitive types (`i8..i64`, `u8..u64`, `f32`, `f64`, `boolean`, `karakter`,
  `teks`, plus aliases `angka`, `bita`, `bool`, `char`, `string`)
- immutable/mutable bindings (`nilai` / `ubah`) with type inference
- functions, recursion, explicit parameter and return types
- arithmetic, comparison, and short-circuit logical operators
- `jika` / `lain jika` / `lain`, `selama`, `untuk … dari … sampai/hingga …`
- rich diagnostics (source spans, error codes, snippets, carets, help notes)
- constant folding + dead-code elimination on Exotickik IR
- a formatter, and a `exotickik` CLI toolchain

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for what is done and what is planned
(structs, generics, enums, pattern matching, ownership/borrow checking, the
standard library, modules across files, LSP, and the package manager).

## Building

Requires a Rust toolchain, plus LLVM's `llc` and a C-ABI linker (`cc`/`clang`)
on `PATH` for producing executables.

```
cargo build --release
```

The `exotickik` binary is produced at `target/release/exotickik`.

## Using the toolchain

```
exotickik run <file.exo>      # compile and run
exotickik build <file.exo>    # compile to a native executable
exotickik check <file.exo>    # type-check only
exotickik fmt <file.exo>      # format source
exotickik emit-ir <file.exo>  # print the generated LLVM IR
exotickik init [nama]            # scaffold a new project
```

See [`docs/TOOLCHAIN.md`](docs/TOOLCHAIN.md) for all commands and options.

## Architecture

```
Exotickik source → lexer → parser → AST → semantic analysis → type checking
            → Exotickik IR → optimization → LLVM IR → llc → object → linker → executable
```

The compiler is a Cargo workspace of focused crates:

| Crate | Responsibility |
|---|---|
| `exotickik_span` | source map, spans, line/column resolution |
| `exotickik_diagnostics` | diagnostic engine, codes, snippet rendering |
| `exotickik_lexer` | tokenizer |
| `exotickik_ast` | abstract syntax tree |
| `exotickik_parser` | recursive-descent + precedence-climbing parser |
| `exotickik_types` | type universe |
| `exotickik_sema` | scopes, resolution, type checking, control-flow checks |
| `exotickik_ir` | Exotickik IR, lowering, optimizer passes |
| `exotickik_codegen` | LLVM IR emission, backend/linker driver, target abstraction |
| `exotickik_fmt` | deterministic formatter |
| `exotickik_driver` | stage orchestration |
| `exotickik_cli` | the `exotickik` command |

Full design documents live in [`docs/`](docs/): `LANGUAGE_SPEC.md`, `GRAMMAR.md`,
`ARCHITECTURE.md`, `MEMORY_MODEL.md`, `TYPE_SYSTEM.md`, `STANDARD_LIBRARY.md`,
`TOOLCHAIN.md`, and `ROADMAP.md`.

## Testing

```
cargo test
```

Unit tests cover each compiler stage; integration and regression tests under
`cli/tests/` compile and run real Exotickik programs through the actual `exotickik` binary
and assert their output, exit codes, and diagnostics.

## Targets

Linux x86-64 is implemented and validated end-to-end. The backend is
target-abstracted (`--target host|linux-x64|windows-x64`); the Windows x64 path
is wired through `lld-link` but not yet runtime-verified. No Windows-specific
assumptions leak outside the target layer.

## License

Dual-licensed under MIT or Apache-2.0.
