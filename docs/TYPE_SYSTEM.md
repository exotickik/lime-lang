# Lime Type System

Lime is statically and (in the safe subset) strongly typed. Types are resolved
and checked entirely at compile time; there is no runtime type information in the
bootstrap.

## Type universe

```
Type =
  | Int   { width: 8|16|32|64, signed: bool }   // i8..i64, u8..u64
  | Float { width: 32|64 }                        // f32, f64
  | Bool                                          // boolean
  | Char                                          // karakter (u32 scalar)
  | Str                                           // teks
  | Unit                                          // hampa
  | Struct(StructId)                              // named product type
  | Enum(EnumId)                                  // named tagged type (i32 tag)
  | Array { elem: Type, size: u64 }               // fixed-size array [T; N]
  | Kotak(Type)                                   // owned heap cell kotak<T>
  | Ref { inner: Type, mutable: bool }            // reference &T / &ubah T (native pointer)
  | Hasil { ok: Type, err: Type }                 // recoverable result Hasil<T, R>
  | Utas(Type)                                    // OS-thread handle Utas<R>
  | Daftar(Type)                                  // growable list daftar<T>
  | Fn { params: [Type], ret: Type }              // function value type fungsi(A)->R
  | Param(String)                                 // generic type variable
  | Ptr                                           // opaque FFI handle (pointer)
  | Error                                         // poison type (post-error)
```

`Ptr` (`pointer`) is an opaque native handle for FFI. It is Copy and may cross
the FFI boundary, but it is not dereferenceable, not castable to or from an
integer, and not printable; it is produced only by an `ekstern` return or
`kosong()` (null) and compared with `==` / `!=`. Because Lime can never read or
write through it, it adds no unsafety. See `docs/LANGUAGE_SPEC.md` §7.

## Copy vs Owned (`salinan` / `milik`)

Every type is classified **Copy** or **Owned**. A type is Owned iff it is
`Kotak(T)` or an aggregate (`Struct`/`Array`) that transitively contains an Owned
type; everything else is Copy. Copy types are copied on assignment/pass/return
and never dropped; Owned types move and are dropped deterministically. See
`docs/MEMORY_MODEL.md` for the full ownership rules. `kotak(x)` builds a
`Kotak(typeof x)`; `isi(k)` reads `T` from a `Kotak(T)` and requires `T` to be
Copy.

`angka` = `Int{64,signed}`, `bita` = `Int{8,unsigned}`, `bool`/`char`/`string`
alias `Bool`/`Char`/`Str`.

The `Error` type is a *poison*: once a subexpression is `Error`, dependent checks
are suppressed to avoid cascading diagnostics.

## Typing rules (core)

- **Literals.** Integer literal → `angka` unless suffixed or constrained by an
  annotation/expected type. Float literal → `f64` similarly. `benar`/`salah` →
  `boolean`. String/char literals → `teks`/`karakter`.
- **Bindings.** `nilai x: T = e` requires `typeof(e)` assignable to `T`.
  `nilai x = e` infers `x : typeof(e)`.
- **Arithmetic** `a ⊕ b` for `⊕ ∈ {+ - * / %}`: requires `typeof(a) == typeof(b)`
  and both numeric. Result type equals the operand type. **No implicit
  widening/narrowing** — `i32 + i64` is `E0210`.
- **Comparison** `< <= > >=`: numeric operands of equal type → `boolean`.
- **Equality** `== !=`: operands of equal type (numeric, `boolean`, `karakter`,
  `teks`) → `boolean`.
- **Logical** `dan atau bukan`: operands must be `boolean` → `boolean`;
  short-circuit evaluation.
- **Unary** `-e`: numeric, same type. `bukan e`: `boolean`.
- **Calls.** `f(a1..an)`: `f` must have function type with arity `n`; each `ai`
  assignable to the declared parameter type; result is the return type.
- **Assignment** `lhs = rhs`: `lhs` must be an assignable l-value bound with
  `ubah`; `typeof(rhs)` assignable to `typeof(lhs)`; result type `hampa`.
- **Field access** `e.f`: `typeof(e)` must be a struct with a field `f` visible
  in the current module; result is the field type.
- **Struct literal** `Nama { f1: e1, ... }`: `Nama` must name a struct type. Each
  named field must exist on that struct, appear exactly once, and its value must
  be assignable to the declared field type. Every declared field must be present.
  The result type is that struct. Missing fields → `E0262`, unknown fields →
  `E0260`, duplicates → `E0263`, a non-struct name → `E0261`.
- **Enum variant** `Enum.Var`: `Enum` must name an enum type and `Var` one of its
  variants; the result type is that enum. Unknown variant → `E0265`. Enum values
  support only `==`/`!=` against the same enum type, and print as the variant
  name; there is no implicit enum↔integer conversion.
- **Array literal** `[e1, ..., ek]`: all elements must share one element type `T`
  (element mismatch → `E0212`); result type `[T; k]`.
- **Index** `e[i]`: `typeof(e)` must be an array (else `E0266`) and `typeof(i)` an
  integer (else `E0211`); result is the element type. Indices are bounds-checked
  at runtime.
- **`panjang(e)`**: `e` must be an array; result is `angka`.
- **`untuk x pada e`**: `e` must be an array (else `E0267`); `x` is bound to the
  element type for the loop body.

## References

A reference is a native, non-owning pointer to an existing value. `&T` is a
shared (read-only) reference; `&ubah T` is a mutable reference. A reference is
always Copy — it is a machine pointer — and it never owns the value it points
at, so borrowing never allocates and never drops.

- **Creation.** `ambil(x)` yields `&typeof(x)`; `ambil_ubah(x)` yields
  `&ubah typeof(x)`. The argument must be a *place* (a variable, field, or index
  access), else `E0290`. `ambil_ubah` additionally requires the place to be
  reachable through a `ubah` binding, else `E0291`.
- **Dereference.** `*r` requires `r : &T` or `&ubah T` (else `E0292`) and has
  type `T`. Reading `*r` copies `T` out, so `T` must be Copy — dereferencing a
  reference to an Owned value (e.g. `&kotak<i64>`) for its value is `E0293`.
- **Write through reference.** `*r = e` requires `r : &ubah T` (a shared `&T`
  target is `E0294`) and `e` assignable to `T`.
- **`isi` through a reference.** `isi` accepts `&kotak<T>` / `&ubah kotak<T>` in
  addition to `kotak<T>`, reading the Copy payload without moving the box.

The borrow checker (see `docs/MEMORY_MODEL.md`) enforces the aliasing and
lifetime rules on top of these typing rules.

## Error handling (`Hasil<T, R>`)

`Hasil<T, R>` is a built-in generic result type: a success carrying a `T` or a
failure carrying an error `R`. It is a value type with a tagged layout, fully
resolved and checked at compile time; there is no exception machinery.

- **Construction.** `sukses(x)` has type `Hasil<typeof x, R>` and `gagal(e)` has
  type `Hasil<T, typeof e>`; the unconstrained parameter is taken from the
  expected type in scope (return type, `let` annotation, or argument). With no
  expected `Hasil` the constructor is `E02B0`; a payload whose type conflicts
  with the expected `Hasil` is `E02B5`.
- **Propagation.** `coba e` requires `e : Hasil<T, R>` (else `E02B1`) and yields
  `T`; on a failure it returns early from the enclosing function, which must
  return `Hasil<U, R>` with the same `R` (else `E02B2`). The error value moves
  into the returned failure — ordinary early-return control flow.
- **Inspection.** `berhasil(r) -> boolean`, `nilai_atau(r, d) -> T`, and
  `gagal_atau(r, d) -> R` read a result without assuming a branch; misuse (a
  non-`Hasil` argument or a mistyped fallback) is `E02B4`.
- **Copy requirement.** `T` and `R` must be Copy (`salinan`) for now, so `Hasil`
  is itself Copy and never dropped; an Owned payload is `E02B3`. Unification
  recurses through `Hasil`, so `Hasil<T, R>` parameters infer in generic
  functions.

## Assignability

The bootstrap uses **invariant, name-based** assignability: `S` is assignable to
`T` iff `S == T` (structurally for primitives, by `StructId` for structs). There
is no subtyping and no implicit coercion. The only way to change a numeric type
is an **explicit cast** `e sebagai T`: it converts between integer types
(sign/zero-extend or truncate), between integers and floats (`sitofp`/`fptosi`),
between float widths, and to/from `karakter`. A cast whose source or target is
not numeric or `karakter` is `E02F0`.

## Control-flow checking

After type checking, `lime_sema` runs a control-flow pass:
- A function with return type `T ≠ hampa` must return on every path (`E0230`).
- `berhenti`/`lanjut` outside a loop are errors (`E0240`).
- Unreachable statements after `kembalikan`/`berhenti`/`lanjut` produce warning
  `W0010`.

## Generics

Generic **functions** are supported:
```lime
fungsi maksimum<T>(a: T, b: T) -> T {
    jika a > b { kembalikan a }
    kembalikan b
}
```
- **Type parameters** are listed in `<...>` after the function name and may be
  used anywhere a type is written in the signature or body.
- **Type-argument inference**: a call carries no explicit type arguments; each
  type parameter is inferred by unifying the declared parameter types against the
  concrete argument types. Unification recurses through `[T; N]` and through
  references, so `&T`/`&ubah T` parameters infer `T` from `&U`/`&ubah U`
  arguments (a shared argument does not satisfy a `&ubah` parameter). Unbound parameters → `E0270`; conflicting bindings for
  one parameter (e.g. calling `maksimum` with an `i32` and an `i64`) → a type
  error. Because literal arguments take their default type (`angka`/`f64`) during
  inference, arguments bound to the same type parameter must already share a
  concrete type; there is no cross-argument literal coercion yet.
- **Monomorphization**: each distinct set of type arguments produces one
  specialized function (mangled `name$T1_T2…`), compiled to native code with no
  boxing and no runtime dispatch. Specialized functions are cached, so recursive
  and repeated instantiations reuse one copy.
- **Instantiation-time checking**: a generic body is type-checked once per
  concrete instantiation, so `a > b` is accepted for numeric `T` and rejected for
  a `T` that has no `>` (reported against the definition, with the call site as a
  secondary note). This is real type checking over a shared, type-substituted
  AST — not textual substitution.

Trait bounds on type parameters (`<T: Sifat>`) are supported — see **Traits**
below. **Not yet:** generic structs/enums, explicit type arguments at the call
site (turbofish). Nested generic calls (a generic calling another generic) are
supported.

## Traits

A **trait** (`sifat`) is a named set of method signatures; an **implementation**
(`implementasi`) provides those methods for a concrete type. Dispatch is fully
**static** — every method call resolves at compile time to an ordinary function,
so traits add no runtime representation and no vtables.

```lime
sifat Luas {
    fungsi luas(diri: &Diri) -> angka
}
implementasi Luas untuk Persegi {
    fungsi luas(diri: &Diri) -> angka { kembalikan diri.sisi * diri.sisi }
}
```

- **Self type.** `Diri` inside a `sifat`/`implementasi` denotes the implementing
  type. The first parameter's form picks the receiver mode: `&Diri` (shared
  borrow), `&ubah Diri` (mutable borrow), or `Diri` (by value).
- **Method calls.** `x.m(a)` resolves `m` among the implementations for the type
  of `x` (auto-dereferencing a reference receiver). Exactly one match is
  required: none → `E02A2`, more than one (two traits) → `E02A3`. The receiver
  is passed as the first argument with the mode its parameter declares, so a
  method call borrows or moves `x` under the same rules as `ambil` /
  `ambil_ubah` / a by-value pass.
- **Impl checking.** A trait impl must define exactly the trait's methods with
  matching signatures (`Diri` substituted by the target type); a missing,
  extra, or mis-typed method is `E02A1`. Implementing the same trait twice for a
  type is `E02A5`; an unknown trait or target type is `E02A6`; a duplicated
  method name inside a `sifat` is `E02A0`. `implementasi Tipe { ... }` (no
  trait) adds inherent methods.
- **Bounds.** A type parameter may carry trait bounds: `fungsi f<T: Luas>(x: &T)`.
  At each call the inferred concrete type must implement every bound (`E02A4`);
  because generic bodies are checked per monomorphized instance, the method
  calls inside then resolve to the concrete impl — static dispatch with no
  overhead.

**Not yet (documented, not faked):** dynamic dispatch / trait objects, default
method bodies, associated types/constants, and blanket or generic impls.

## Error codes (type system excerpt)

| Code | Meaning |
|---|---|
| E0201 | penggunaan nama yang belum didefinisikan |
| E0202 | pendefinisian ulang nama pada scope yang sama |
| E0210 | ketidakcocokan tipe pada operator biner |
| E0211 | tipe operand tidak valid untuk operator |
| E0212 | ketidakcocokan tipe pada penetapan/inisialisasi |
| E0220 | jumlah argumen tidak sesuai |
| E0221 | pemanggilan nilai yang bukan fungsi |
| E0230 | fungsi tidak mengembalikan nilai pada semua jalur |
| E0231 | tipe nilai kembalian tidak sesuai |
| E0240 | 'berhenti'/'lanjut' di luar loop |
| E0250 | penetapan ke binding immutable ('nilai') |
| E0260 | akses field yang tidak ada pada struct |
| E0261 | nilai yang diakses/dibangun bukan struct |
| E0262 | ada field struct yang belum diisi pada literal |
| E0263 | field struct diisi lebih dari sekali pada literal |
| E0265 | varian enum tidak dikenal |
| E0266 | nilai bukan larik tidak bisa diindeks |
| E0267 | nilai bukan larik tidak bisa diiterasi ('pada') |
| E0270 | parameter tipe generik tidak bisa disimpulkan |
| E0280 | nilai ini sudah dipindahkan |
| E0281 | nilai dipindahkan pada sebagian cabang saja |
| E0282 | nilai luar dipindahkan di dalam perulangan |
| E0283 | tipe milik melewati batas FFI |
| E0284 | nilai bertipe milik ditetapkan ulang |
| E0290 | ambil/ambil_ubah pada sesuatu yang bukan variabel/field/elemen |
| E0291 | ambil_ubah pada binding immutable ('nilai') |
| E0292 | '*' pada nilai yang bukan reference |
| E0293 | membaca nilai bertipe milik lewat reference |
| E0294 | menulis lewat reference '&' (bukan '&ubah') |
| E0295 | reference bentrok (mutable vs reference lain) |
| E0296 | nilai diubah padahal masih dipakai reference |
| E0297 | reference keluar dari scope nilainya (kembalian/struct/larik) |
| E0298 | reference melewati batas FFI |
| E02A0 | metode ditulis dua kali di dalam sifat |
| E02A1 | implementasi tidak cocok dengan sifat (metode kurang/lebih/beda tipe) |
| E02A2 | metode tidak ditemukan untuk tipe penerima |
| E02A3 | metode ambigu (didefinisikan lebih dari satu sifat) |
| E02A4 | tipe tidak memenuhi batas sifat |
| E02A5 | implementasi ganda sebuah sifat untuk satu tipe |
| E02A6 | sifat atau tipe target tidak dikenal |
| E02B0 | `sukses`/`gagal` tanpa tipe `Hasil` yang diharapkan |
| E02B1 | `coba` pada sesuatu yang bukan `Hasil` |
| E02B2 | `coba` di luar fungsi `Hasil<_, R>` atau tipe error tak cocok |
| E02B3 | muatan `Hasil<T, R>` bertipe milik (belum didukung) |
| E02B4 | penyalahgunaan `berhasil`/`nilai_atau`/`gagal_atau` |
| E02B5 | nilai `sukses`/`gagal` bentrok dengan `Hasil` yang diharapkan |
| E02C0 | akses item lintas modul yang tidak `publik` atau belum di-`impor` |
| E02D0 | penyalahgunaan `mulai`/`tunggu` (pekerja/argumen/handle salah) |
| E02E0 | penyalahgunaan builtin `daftar` (arity/penerima/elemen milik) |
| E02F0 | konversi 'sebagai' tidak valid (bukan numerik/karakter) |
