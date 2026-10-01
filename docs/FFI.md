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
| `pointer` | `ptr` | `void*` / any opaque handle (`FILE*`, …) |
| `fungsi(A) -> R` | `ptr` | function pointer `R (*)(A)` (callback) |
| `hampa` | `void` | `void` |

Passing Lime structs and arrays by value across the FFI follows the target C ABI
for aggregates. Owned types (`E0297`) and Lime references (`E0298`) may not cross
the boundary, which keeps Lime's memory guarantees.

## Callbacks

Give an `ekstern` parameter a function type to pass a Lime function where C
expects a function pointer:

```lime
ekstern fungsi atexit(f: fungsi()) -> i32
fungsi pamit() { tampilkan("selesai") }
utama { atexit(pamit) }
```

The Lime function is lowered to a bare function pointer, which C invokes directly
over the ABI. The callback's own parameters and return follow the same type
mapping as above.

## Opaque pointers (`pointer`)

`pointer` is an opaque native handle for binding C APIs that pass pointers (file
handles, allocator handles, library contexts). It is Copy and crosses the FFI
boundary, but it is opaque by design: it cannot be dereferenced, cast to or from
an integer, or printed. A `pointer` comes only from an `ekstern` return or from
`kosong()` (the null pointer), and is compared with `==` / `!=` (usually against
`kosong()`), so Lime never reads or writes through it and stays memory-safe.

```lime
ekstern fungsi fopen(nama: teks, mode: teks) -> pointer
ekstern fungsi fclose(f: pointer) -> i32
utama {
    nilai f = fopen("data.txt", "r")
    jika f == kosong() { tampilkan("gagal") } lain { nilai _ = fclose(f) }
}
```

## Linking

No extra configuration is needed for symbols already provided by the C runtime
(`abs`, `putchar`, `printf`, …): the Lime backend links the final executable with
the system C toolchain driver (`cc`/`clang`), which links libc by default.
Linking additional native libraries (custom link flags, `-l`) is planned work in
the build system / package manager.

## Status

Implemented and tested: declaring non-variadic and variadic externs, calling
them with type checking, linking against libc, callbacks (Lime functions as C
function pointers), and the opaque `pointer` type with `kosong()`. See
`cli/tests/ffi.rs` and `examples/pointer_ekstern.lime`. Not yet available: passing
Lime references/buffers across the boundary (forbidden by design for safety; a
future owned-buffer or explicit-unsafe mechanism may lift this), and per-package
link configuration (custom `-l` flags).
