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
  | Fn { params: [Type], ret: Type }              // function type
  | Param(String)                                 // generic type variable
  | Error                                         // poison type (post-error)
```

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

## Assignability

The bootstrap uses **invariant, name-based** assignability: `S` is assignable to
`T` iff `S == T` (structurally for primitives, by `StructId` for structs). There
is no subtyping and no implicit coercion. Explicit conversions (`sebagai`, a
`cast` operator) are **planned** and will be the only way to change numeric type.

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
  concrete argument types. Unbound parameters → `E0270`; conflicting bindings for
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

**Not yet:** generic structs/enums, explicit type arguments at the call site
(turbofish), and trait/constraint bounds (planned as the traits phase). Nested
generic calls (a generic calling another generic) are supported.

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
| E0266 | nilai bukan larik tidak dapat diindeks |
| E0267 | nilai bukan larik tidak dapat diiterasi ('pada') |
| E0270 | parameter tipe generik tidak dapat disimpulkan |
| E0280 | nilai digunakan setelah dipindahkan |
| E0281 | nilai dipindahkan pada sebagian cabang saja |
| E0282 | nilai luar dipindahkan di dalam perulangan |
| E0283 | tipe milik melewati batas FFI |
| E0284 | penetapan ulang nilai bertipe milik |
