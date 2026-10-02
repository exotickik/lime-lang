# EXOTICKIK — Native General-Purpose Programming Language

You are the lead compiler engineer responsible for creating **Exotickik**, a completely new general-purpose programming language.

This is NOT a C++ wrapper.

This is NOT a C++ transpiler.

This is NOT a syntax translator.

This is NOT a language that generates C or C++ code.

Exotickik must be designed as an independent programming language with its own:

* syntax
* grammar
* lexer
* parser
* AST
* semantic analysis
* type system
* memory model
* module system
* package system
* standard library
* compiler
* intermediate representation
* optimizer pipeline
* native code generation
* diagnostics
* formatter
* language server
* build system

The final compiler must produce **native machine code**.

---

# 1. CORE VISION

Exotickik is intended to be a serious general-purpose programming language capable of building:

* desktop applications
* games
* servers
* CLI applications
* networking software
* developer tools
* system utilities
* libraries
* performance-sensitive applications

The long-term performance target is the class of languages such as C++, Rust, and other native compiled languages.

Do NOT sacrifice the language architecture merely to make the first prototype easier.

The first implementation may be incomplete, but its architecture must be designed so Exotickik can eventually become a serious production language.

---

# 2. ABSOLUTE RESTRICTIONS

These rules are mandatory.

## DO NOT:

* transpile Exotickik into C
* transpile Exotickik into C++
* generate C++ as an intermediate representation
* generate C as an intermediate representation
* implement Exotickik as a Python/JavaScript interpreter
* implement Exotickik as a wrapper around another language
* fake compilation
* create placeholder compiler stages that pretend to generate native code

LLVM may be used as a **backend infrastructure and optimization/code-generation component**.

Using LLVM does NOT mean Exotickik is based on C++ syntax or semantics.

Exotickik's frontend, language semantics, AST, type system, IR design, diagnostics, and language rules must belong to Exotickik.

---

# 3. LANGUAGE DESIGN

Before implementing the compiler, design the language itself.

Do not simply copy C++ syntax.

Exotickik should have a clean, modern, readable syntax.

The syntax may take inspiration from multiple languages, but Exotickik must have its own coherent design.

For example, Exotickik could look conceptually like:

```exotickik
program Halo

utama {
    tampilkan("Halo dunia!")
}
```

Variables:

```exotickik
nilai nama = "Exotickik"
nilai umur: angka = 20
nilai aktif: boolean = benar
```

Functions:

```exotickik
fungsi tambah(a: angka, b: angka) -> angka {
    kembalikan a + b
}
```

Conditions:

```exotickik
jika umur >= 18 {
    tampilkan("Dewasa")
} lain {
    tampilkan("Belum dewasa")
}
```

Loops:

```exotickik
untuk nilai i dari 0 sampai 10 {
    tampilkan(i)
}
```

These examples are NOT a fixed specification.

You are responsible for designing a consistent grammar.

Do not blindly copy these examples if they create poor language design.

---

# 4. LANGUAGE IDENTITY

Exotickik must have a clear identity.

Design:

## Primitive types

At minimum:

* integer types
* floating-point types
* boolean
* character
* string
* byte

Support explicit integer sizes where appropriate:

```text
i8
i16
i32
i64
u8
u16
u32
u64
```

and floating point:

```text
f32
f64
```

Provide sensible aliases if desired.

---

# 5. MEMORY MODEL

This is extremely important.

Exotickik is intended to compete in performance-sensitive native software.

Do NOT automatically introduce a garbage collector.

Design a predictable native memory model.

Evaluate and document a suitable approach such as:

* ownership
* borrowing
* move semantics
* deterministic destruction
* RAII-style resource management
* explicit allocation
* stack allocation
* heap allocation

The final design must minimize unnecessary runtime overhead.

The compiler should be capable of determining when values can be stored on the stack.

Avoid hidden allocations whenever possible.

---

# 6. ZERO-COST ABSTRACTIONS

Exotickik should aim for abstractions that compile efficiently.

Examples:

```exotickik
fungsi maksimum(a: angka, b: angka) -> angka {
    jika a > b {
        kembalikan a
    }

    kembalikan b
}
```

The compiler should be able to inline small functions when appropriate.

Design the language so abstraction does not automatically mean runtime overhead.

---

# 7. GENERICS

Exotickik should eventually support generic programming.

Conceptually:

```exotickik
fungsi maksimum<T>(a: T, b: T) -> T {
    ...
}
```

Do not implement this merely as textual substitution.

Design proper generic semantics.

The architecture should allow monomorphization or another efficient native strategy.

---

# 8. STRUCTS / DATA TYPES

Exotickik needs user-defined data structures.

For example:

```exotickik
tipe Pemain {
    nama: teks
    skor: i32
}
```

Methods:

```exotickik
tipe Pemain {
    nama: teks
    skor: i32

    fungsi tambah_skor(jumlah: i32) {
        skor = skor + jumlah
    }
}
```

Design a coherent system for:

* structs
* methods
* constructors
* visibility
* composition
* enums
* pattern matching if appropriate

Do not automatically copy C++ classes/inheritance.

Prefer a simpler modern design unless there is a strong reason otherwise.

---

# 9. MODULE SYSTEM

Exotickik needs a real module system.

Example:

```exotickik
gunakan matematika
gunakan jaringan
gunakan filesystem
```

And user modules:

```exotickik
impor utilitas
```

Design:

* namespaces/modules
* public/private visibility
* module resolution
* dependency handling
* circular dependency detection

---

# 10. PACKAGE MANAGER

Exotickik should eventually have its own package manager.

For example:

```text
exotickik init
exotickik build
exotickik run
exotickik test
exotickik tambah paket
exotickik hapus paket
exotickik perbarui
```

Package definitions should be designed from the beginning even if the registry is implemented later.

---

# 11. NATIVE COMPILATION

The compiler architecture must be:

```text
Exotickik Source
     ↓
Lexer
     ↓
Parser
     ↓
AST
     ↓
Semantic Analysis
     ↓
Type Checking
     ↓
Exotickik IR
     ↓
Optimization
     ↓
Native Code Generation
     ↓
Object Files
     ↓
Linker
     ↓
Native Executable
```

Exotickik IR must be conceptually independent from the source syntax.

LLVM may be used after Exotickik's own semantic/IR stages.

Possible architecture:

```text
Exotickik AST
   ↓
Exotickik Semantic IR
   ↓
Exotickik Optimization
   ↓
LLVM IR
   ↓
LLVM Optimization
   ↓
Machine Code
```

Do NOT convert Exotickik source directly into C++.

---

# 12. TARGET PLATFORMS

Initial priority:

**Windows x64**

The architecture must allow:

* Windows x64
* Linux x64
* Linux ARM64
* macOS ARM64

to be added later.

Do not hardcode Windows-specific assumptions throughout the compiler.

---

# 13. C / C++ INTEROPERABILITY

C/C++ interoperability is allowed ONLY as an external FFI.

Example:

```exotickik
ekstern fungsi printf(...)
```

or another clean Exotickik FFI design.

This means:

```text
Exotickik → native ABI → C library
```

NOT:

```text
Exotickik → C++ source → compiler
```

Exotickik must remain an independent language.

---

# 14. STANDARD LIBRARY

Create a modular standard library.

Initial areas:

```text
exotickik/std/
├── io
├── string
├── collections
├── filesystem
├── process
├── math
├── time
├── threading
├── networking
└── system
```

Do not implement everything immediately.

Create stable interfaces and expand incrementally.

---

# 15. COMPILER CLI

The compiler executable should eventually support:

```text
exotickik build main.exo
exotickik run main.exo
exotickik check main.exo
exotickik test
exotickik fmt
exotickik init
exotickik clean
exotickik tambah
exotickik --version
exotickik --help
```

The command-line interface should feel like a professional compiler toolchain.

---

# 16. ERROR MESSAGES

Diagnostics are a major part of the language.

Bad:

```text
error
```

Good:

```text
main.exo:12:18: error: variabel 'umur' belum didefinisikan

12 | tampilkan(umur)
                 ^^^^

variabel tersebut belum tersedia pada scope ini
```

Implement:

* source locations
* line/column
* source snippets
* error codes
* warnings
* notes
* suggestions where possible

Design the diagnostic engine as a reusable compiler subsystem.

---

# 17. FORMATTER

Create:

```text
exotickik fmt
```

The formatter must parse Exotickik source rather than performing dangerous text replacements.

Formatting should be deterministic.

---

# 18. LSP

After compiler fundamentals are stable, implement a Language Server.

Support:

* diagnostics
* autocomplete
* go-to-definition
* symbol search
* hover information
* document formatting
* basic rename support

Create a VS Code extension.

---

# 19. TESTING

This project must be heavily tested.

Create:

```text
tests/
├── lexer/
├── parser/
├── semantic/
├── typecheck/
├── ir/
├── codegen/
├── diagnostics/
├── stdlib/
├── integration/
└── regression/
```

Test:

* valid programs
* invalid programs
* parser edge cases
* type errors
* ownership errors
* malformed syntax
* compiler crashes
* generated executables
* runtime behavior
* optimization correctness

Every discovered compiler bug MUST receive a regression test.

---

# 20. COMPILER VALIDATION

Do not trust the compiler merely because it builds.

Create test programs such as:

```exotickik
program Hello

utama {
    tampilkan("Hello World")
}
```

Compile it to a real Windows executable.

Run it.

Verify its output.

Then progressively test:

* arithmetic
* functions
* variables
* conditionals
* loops
* structs
* arrays
* strings
* modules
* generics
* memory management
* filesystem
* networking
* concurrency

---

# 21. PERFORMANCE

Performance is a first-class goal.

Create benchmarks for:

* compile time
* executable size
* runtime performance
* memory usage
* allocation count
* startup time

Do not make unsupported claims such as:

> "Exotickik is faster than C++."

Instead, create reproducible benchmarks.

Compare Exotickik against equivalent implementations in established languages when useful.

---

# 22. PROJECT STRUCTURE

Use a modular architecture.

Suggested structure:

```text
exotickik/
├── compiler/
│   ├── lexer/
│   ├── parser/
│   ├── ast/
│   ├── diagnostics/
│   ├── semantic/
│   ├── types/
│   ├── ir/
│   ├── optimizer/
│   ├── codegen/
│   └── linker/
│
├── runtime/
├── stdlib/
├── cli/
├── formatter/
├── lsp/
├── vscode/
├── package-manager/
│
├── tests/
├── examples/
├── benchmarks/
│
├── docs/
│   ├── LANGUAGE_SPEC.md
│   ├── ARCHITECTURE.md
│   ├── MEMORY_MODEL.md
│   ├── TYPE_SYSTEM.md
│   ├── STANDARD_LIBRARY.md
│   ├── TOOLCHAIN.md
│   └── ROADMAP.md
│
└── README.md
```

You may change this structure if a better architecture is justified.

---

# 23. DEVELOPMENT PHASES

Do NOT attempt to implement everything at once.

## Phase 0 — Language Design

Before substantial implementation:

* define Exotickik identity
* define grammar
* define type system
* define memory model
* define module system
* define error model
* define compilation model

Create:

```text
docs/LANGUAGE_SPEC.md
docs/ARCHITECTURE.md
docs/MEMORY_MODEL.md
docs/TYPE_SYSTEM.md
docs/ROADMAP.md
```

Get the architecture internally consistent before proceeding.

---

## Phase 1 — Compiler Foundation

Implement:

* project build system
* lexer
* parser
* AST
* source locations
* diagnostics
* basic CLI
* `exotickik check`

At the end:

```text
exotickik check hello.exo
```

must actually parse and validate Exotickik source.

---

## Phase 2 — Semantic System

Implement:

* symbol tables
* scopes
* type checking
* functions
* variables
* expressions
* control flow
* structs
* arrays

---

## Phase 3 — Native Backend

Implement:

* Exotickik IR
* lowering
* LLVM integration if selected
* object generation
* linker integration
* Windows x64 executable generation

At the end:

```text
exotickik build hello.exo
```

must produce a real executable.

---

## Phase 4 — Runtime + Standard Library

Implement:

* IO
* strings
* collections
* filesystem
* memory utilities
* process handling

---

## Phase 5 — Advanced Language

Implement progressively:

* generics
* enums
* pattern matching
* error handling
* modules
* concurrency
* FFI

Only implement features after their semantics are properly specified.

---

## Phase 6 — Developer Tooling

Implement:

* formatter
* LSP
* VS Code extension
* debugger integration where practical
* documentation tooling

---

## Phase 7 — Package Ecosystem

Implement:

* package manager
* dependency resolution
* package manifest
* local packages
* package cache
* build profiles

---

## Phase 8 — Optimization

Implement and benchmark:

* constant folding
* dead code elimination
* inlining
* loop optimizations
* escape analysis where appropriate
* allocation optimization
* link-time optimization where applicable

Only retain optimizations that preserve correctness.

---

# 24. DEVELOPMENT RULES

You are working on a real compiler project.

Therefore:

1. Inspect the existing repository before modifying anything.
2. Do not overwrite existing work unnecessarily.
3. Keep architecture modular.
4. Avoid giant files.
5. Avoid TODO-driven fake implementations.
6. Never claim functionality exists if it is not implemented.
7. Run tests after meaningful changes.
8. Fix failures instead of hiding them.
9. Add regression tests for bugs.
10. Keep documentation synchronized with implementation.
11. Prefer correctness over rushing through phases.
12. Do not introduce unnecessary dependencies.
13. Keep the compiler usable from a clean environment.
14. Do not require internet access during normal compilation.

---

# 25. IMPORTANT AUTONOMOUS WORKFLOW

Use this loop continuously:

```text
Design
 ↓
Implement
 ↓
Build
 ↓
Test
 ↓
Find failures
 ↓
Fix
 ↓
Regression test
 ↓
Benchmark
 ↓
Document
```

Do not stop at creating files.

A feature is considered complete only when it:

* has implementation
* has tests
* passes tests
* integrates with the compiler
* has documentation
* works in a real example

---

# 26. FIRST TASK

Do NOT immediately start implementing random compiler files.

First:

1. Inspect the repository.
2. Inspect the available build environment.
3. Determine available compiler/toolchain dependencies.
4. Design Exotickik's language architecture.
5. Write the initial language specification.
6. Write the compiler architecture.
7. Define the memory model.
8. Define the type system.
9. Define the compilation pipeline.
10. Define the initial grammar.
11. Create the development roadmap.
12. Then begin Phase 1.

The final objective is:

> **Exotickik should become a real independent native programming language, not a C++ clone and not a C++ translator.**

Build the foundation correctly so the project can eventually grow into a serious programming ecosystem.
