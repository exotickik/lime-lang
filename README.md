# Lime

Lime is an independent, statically-typed, natively-compiled, general-purpose
programming language with a clean, modern syntax whose keywords are Indonesian.

Lime is **not** a C/C++ wrapper, transpiler, or macro layer. It has its own
lexer, parser, AST, semantic analysis, type system, SSA-style intermediate
representation, optimizer, and native code generation. LLVM is used strictly as
a backend: Lime lowers its own IR to LLVM IR, which `llc` turns into real machine
code. No stage ever emits or consumes C or C++.

```lime
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
$ lime run examples/fibonacci.lime
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
type-checks, lowers to a Lime-owned IR, optimizes, emits LLVM IR, and links a
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
- constant folding + dead-code elimination on Lime IR
- a formatter, and a `lime` CLI toolchain

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for what is done and what is planned
(structs, generics, enums, pattern matching, ownership/borrow checking, the
standard library, modules across files, LSP, and the package manager).

## Building

Requires a Rust toolchain, plus LLVM's `llc` and a C-ABI linker (`cc`/`clang`)
on `PATH` for producing executables.

```
cargo build --release
```

The `lime` binary is produced at `target/release/lime`.

## Using the toolchain

```
lime run <berkas.lime>      # compile and run
lime build <berkas.lime>    # compile to a native executable
lime check <berkas.lime>    # type-check only
lime fmt <berkas.lime>      # format source
lime emit-ir <berkas.lime>  # print the generated LLVM IR
lime init [nama]            # scaffold a new project
```

See [`docs/TOOLCHAIN.md`](docs/TOOLCHAIN.md) for all commands and options.

## Architecture

```
Lime source → lexer → parser → AST → semantic analysis → type checking
            → Lime IR → optimization → LLVM IR → llc → object → linker → executable
```

The compiler is a Cargo workspace of focused crates:

| Crate | Responsibility |
|---|---|
| `lime_span` | source map, spans, line/column resolution |
| `lime_diagnostics` | diagnostic engine, codes, snippet rendering |
| `lime_lexer` | tokenizer |
| `lime_ast` | abstract syntax tree |
| `lime_parser` | recursive-descent + precedence-climbing parser |
| `lime_types` | type universe |
| `lime_sema` | scopes, resolution, type checking, control-flow checks |
| `lime_ir` | Lime IR, lowering, optimizer passes |
| `lime_codegen` | LLVM IR emission, backend/linker driver, target abstraction |
| `lime_fmt` | deterministic formatter |
| `lime_driver` | stage orchestration |
| `lime_cli` | the `lime` command |

Full design documents live in [`docs/`](docs/): `LANGUAGE_SPEC.md`, `GRAMMAR.md`,
`ARCHITECTURE.md`, `MEMORY_MODEL.md`, `TYPE_SYSTEM.md`, `STANDARD_LIBRARY.md`,
`TOOLCHAIN.md`, and `ROADMAP.md`.

## Testing

```
cargo test
```

Unit tests cover each compiler stage; integration and regression tests under
`cli/tests/` compile and run real Lime programs through the actual `lime` binary
and assert their output, exit codes, and diagnostics.

## Targets

Linux x86-64 is implemented and validated end-to-end. The backend is
target-abstracted (`--target host|linux-x64|windows-x64`); the Windows x64 path
is wired through `lld-link` but not yet runtime-verified. No Windows-specific
assumptions leak outside the target layer.

## License

Dual-licensed under MIT or Apache-2.0.
