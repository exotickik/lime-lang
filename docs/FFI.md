# Lime Foreign Function Interface (FFI)

Lime calls native C-ABI libraries through an explicit FFI. This is an
**interoperability mechanism only** — it is not Lime's compilation strategy.
Lime source always compiles through Lime's own pipeline
(lexer → parser → semantics → Lime IR → LLVM IR → native code); the FFI simply
lets that native code call symbols provided by other native libraries at link
time. Lime never emits or consumes C/C++ source.

## Declaring an external function

```lime
ekstern fungsi abs(x: i32) -> i32
ekstern fungsi putchar(c: i32) -> i32
ekstern fungsi printf(fmt: teks, ...) -> i32
```

An `ekstern fungsi` declaration names a function that exists in a linked native
library, with Lime parameter and return types. It has no body. A missing `-> T`
means the function returns `hampa` (C `void`).

Variadic C functions are declared with a trailing `...` after the fixed
parameters, as in `printf` above.

## Calling

External functions are called exactly like Lime functions and are type-checked
the same way:

```lime
utama {
    nilai n: i32 = 0i32 - 42i32
    tampilkan(abs(n))                 // 42
    printf("x=%d y=%d\n", 3i32, 4i32) // x=3 y=4
}
```

For a variadic extern, the fixed arguments are checked against the declared
parameter types and any additional arguments are passed through with their
natural types.

## ABI and type mapping

Lime lowers an `ekstern` declaration to an LLVM `declare` and lowers calls to
LLVM `call`, so the platform C ABI governs argument passing and return values.
The Lime → LLVM type mapping used at the boundary:

| Lime | LLVM | C (typical) |
|---|---|---|
| `i8 i16 i32 i64` | `i8 i16 i32 i64` | `int8_t … int64_t` |
| `u8 u16 u32 u64` | `i8 i16 i32 i64` | `uint8_t … uint64_t` |
| `f32` / `f64` | `float` / `double` | `float` / `double` |
| `boolean` | `i1` | `bool` |
| `karakter` | `i32` | `int32_t` (Unicode scalar) |
| `teks` | `ptr` | `const char*` (NUL-terminated, bootstrap) |
| `hampa` | `void` | `void` |

Passing Lime structs and arrays by value across the FFI follows the target C ABI
for aggregates; passing pointers/references to them awaits the references phase.

## Linking

No extra configuration is needed for symbols already provided by the C runtime
(`abs`, `putchar`, `printf`, …): the Lime backend links the final executable with
the system C toolchain driver (`cc`/`clang`), which links libc by default.
Linking additional native libraries (custom link flags, `-l`) is planned work in
the build system / package manager.

## Status

Implemented and tested: declaring non-variadic and variadic externs, calling
them with type checking, and linking against libc. See `cli/tests/ffi.rs` and
`examples/ffi_libc.lime`. Not yet available: passing aggregates by reference,
callbacks (Lime functions as C function pointers), and per-package link
configuration.
