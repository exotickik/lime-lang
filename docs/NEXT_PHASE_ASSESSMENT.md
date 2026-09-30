# Phase 12 (Error Handling) — Design Assessment

Baseline: `main` at the Traits checkpoint — 197 tests, 0 warnings, all examples
run, formatter idempotent. Ownership, references, generics, and traits must not
regress.

## Goal

A real, compile-time-enforced recoverable-error model designed for Lime — not a
copy of Rust's `?`, Go's `(v, err)`, or C++ exceptions. Errors are ordinary
values; there is no unwinding, no runtime exception machinery, and no hidden
control flow. Propagation is explicit and visible in the source.

## Model

A recoverable result is the built-in generic type **`Hasil<T, R>`** ("hasil" =
result): a value that is either a success carrying a `T` or a failure carrying an
`R` (the error value/type). It is a first-class, compiler-supported generic type
in the same spirit as `kotak<T>`, with a real tagged layout — not a library
convention that the compiler does not understand.

```lime
fungsi bagi(a: angka, b: angka) -> Hasil<angka, teks> {
    jika b == 0 {
        kembalikan gagal("pembagian dengan nol")
    }
    kembalikan sukses(a / b)
}
```

### Construction

- `sukses(x)` builds a success `Hasil<T, R>` with `T = typeof x`.
- `gagal(e)` builds a failure `Hasil<T, R>` with `R = typeof e`.

The *other* type parameter is fixed by the surrounding expected type (the
function return type, a `let` annotation, or an argument type), exactly like an
empty array literal. With no expected `Hasil` type in scope the constructor is a
`cannot-infer` error — the model is never silently untyped.

### Propagation — `coba`

`coba e` ("coba" = try) is the propagation operator. `e` must be a
`Hasil<T, R>`. If `e` is a success, `coba e` evaluates to the unwrapped `T`. If
`e` is a failure, the enclosing function **returns early** with that failure. The
enclosing function must itself return `Hasil<U, R>` with the *same* error type
`R`; the error value moves into the returned failure. Propagation is therefore
ordinary, visible early-return control flow — no stack unwinding.

```lime
fungsi hitung(a: angka, b: angka, c: angka) -> Hasil<angka, teks> {
    nilai x = coba bagi(a, b)     // unwraps, or returns the failure
    nilai y = coba bagi(x, c)
    kembalikan sukses(y)
}
```

### Inspection at a boundary

Where propagation is not wanted (e.g. `utama`, which returns an exit code), a
result is inspected with three builtins that never assume a branch:

- `berhasil(r) -> boolean` — is `r` a success?
- `nilai_atau(r, cadangan) -> T` — the success value, or `cadangan` on failure.
- `gagal_atau(r, cadangan) -> R` — the error value, or `cadangan` on success.

Both branches are thus reachable safely without unchecked extraction:

```lime
nilai r = bagi(10, 0)
jika berhasil(r) {
    tampilkan(nilai_atau(r, 0))
} lain {
    tampilkan(gagal_atau(r, "?"))
}
```

## Type-system integration

- New `Type::Hasil { ok, err }`. Display `Hasil<T, R>`.
- Assignability stays invariant: `Hasil<A, B>` is assignable only to
  `Hasil<A, B>`.
- Generic unification recurses through `Hasil` (both parameters), so
  `Hasil<T, R>` parameters in generic functions infer `T` and `R`.

## Ownership / references interaction

For this phase `Hasil<T, R>` requires `T` and `R` to be **Copy** (`salinan`).
This keeps the type Copy, so it is never dropped and needs no *conditional* drop
(dropping the ok payload only on success, the err payload only on failure). Using
an Owned (`milik`) type as either payload is a clear diagnostic, not silent
mis-handling. `teks`, all numbers, `boolean`, and Copy structs/enums cover the
canonical cases (`Hasil<angka, teks>`). Conditional drop for Owned payloads is a
documented later extension, gated behind this phase rather than faked now.

Because `Hasil` is Copy, `sukses`/`gagal`/`coba`/`berhasil`/`nilai_atau`/
`gagal_atau` copy their operands; there are no moves or borrows to track beyond
the existing rules.

## Lowering (native, zero hidden cost)

`Hasil<T, R>` lowers to the LLVM aggregate `{ i32 tag, T ok, R err }` (tag 0 =
success, 1 = failure), reusing the existing struct `insertvalue`/`extractvalue`
machinery — no new aggregate concept:

- `sukses(x)` → `{ 0, x, undef }`; `gagal(e)` → `{ 1, undef, e }`.
- `berhasil(r)` → `icmp eq (extract tag), 0`.
- `nilai_atau`/`gagal_atau` → `select` on the tag.
- `coba e` → extract tag; on failure, construct `{ 1, undef, extract err }` of the
  enclosing return type and `ret`; on success, continue with `extract ok`. This
  is a real branch and early return in the IR, `llc`-optimized like any other.

## Diagnostics (E02B0–E02B5)

| Code | Meaning |
|---|---|
| E02B0 | `sukses`/`gagal` used where the surrounding `Hasil` type is unknown |
| E02B1 | `coba` on something that is not a `Hasil` |
| E02B2 | `coba` outside a function returning `Hasil<_, R>` / mismatched error type |
| E02B3 | `Hasil<T, R>` payload type is not Copy (Owned not yet allowed) |
| E02B4 | inspection builtin (`berhasil`/`nilai_atau`/`gagal_atau`) misused |
| E02B5 | `sukses`/`gagal` value type conflicts with the expected `Hasil` |

## Non-goals this phase (documented, not faked)

- Owned (`milik`) payloads inside `Hasil` (needs conditional drop).
- `cocok` pattern-matching destructuring of `Hasil` (needs general enum
  payloads + pattern matching — a separate roadmap item); the inspection
  builtins cover both branches meanwhile.
- A trait-based error interface (e.g. a `Kesalahan` trait) — errors are plain
  values for now.
