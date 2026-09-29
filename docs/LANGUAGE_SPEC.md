# Lime Language Specification (Bootstrap Edition)

> Status: **living document**. This describes the language as implemented by the
> bootstrap compiler (`limec`) plus the immediate design intent. Sections marked
> **(planned)** are specified but not yet implemented in the bootstrap.

Lime is an independent, statically-typed, natively-compiled, general-purpose
programming language. Its keywords are Indonesian — this is a deliberate identity
choice, not a translation layer: the grammar, semantics, IR and toolchain are
Lime's own.

Lime is **not** a C/C++ wrapper, transpiler or macro layer. The compiler lowers a
Lime-owned AST to a Lime-owned SSA IR, and only then to LLVM IR for machine-code
generation. LLVM is used strictly as a backend.

---

## 1. Lexical structure

### 1.1 Encoding
Source files are UTF-8. The recommended extension is `.lime`.

### 1.2 Comments
```
// line comment
/* block comment (may /* nest */ ) */
```
Block comments nest.

### 1.3 Identifiers
`ident = (letter | '_') (letter | digit | '_')*` where *letter* is any Unicode
alphabetic character. Identifiers are case-sensitive.

### 1.4 Keywords
Reserved and cannot be used as identifiers:

```
program  utama    fungsi   kembalikan
nilai    ubah
jika     lain     selama   untuk    dari   sampai   hingga
berhenti    lanjut
benar    salah
tipe     enum     cocok
gunakan  impor    ekstern  publik
dan      atau     bukan
```

Type names (`i8..i64`, `u8..u64`, `f32`, `f64`, `boolean`, `karakter`, `teks`,
`bita`, `hampa`, `angka`) are **contextual**: they are ordinary identifiers that
resolve to built-in types, so they are not reserved words but are predefined in
the root scope.

### 1.5 Literals
| Kind | Examples | Default type |
|---|---|---|
| Integer | `0`, `42`, `1_000`, `0xFF`, `0b1010`, `0o17` | `angka` (= `i64`) |
| Float | `3.14`, `1.0`, `2.5e-3` | `f64` |
| Boolean | `benar`, `salah` | `boolean` |
| Character | `'a'`, `'\n'`, `'\u{1F34B}'` | `karakter` |
| String | `"halo\n"`, `"emoji 🍋"` | `teks` |

Underscores may separate digits in numeric literals. Integer literals may carry
a suffix (`10i32`, `255u8`, `3.0f32`) to fix their type.

Escape sequences in strings/chars: `\n \r \t \\ \" \' \0 \u{XXXX}`.

---

## 2. Types

### 2.1 Primitive types
| Lime | Meaning | LLVM lowering |
|---|---|---|
| `i8 i16 i32 i64` | signed integers | `i8 i16 i32 i64` |
| `u8 u16 u32 u64` | unsigned integers | `i8 i16 i32 i64` (signedness tracked by Lime) |
| `f32 f64` | IEEE-754 floats | `float double` |
| `boolean` | true/false | `i1` |
| `karakter` | Unicode scalar value | `i32` |
| `bita` | raw byte (alias of `u8`) | `i8` |
| `teks` | string (see §2.3) | `i8*` (bootstrap) / `{ptr,len}` (planned) |
| `hampa` | unit / no value | `void` |

Aliases available in the root scope: `angka` = `i64`, `bool` = `boolean`,
`char` = `karakter`, `string` = `teks`.

Signedness is part of the Lime type but not of the LLVM integer type; the code
generator selects signed vs unsigned instructions (`sdiv`/`udiv`,
`icmp slt`/`icmp ult`, `sext`/`zext`) from the Lime type.

### 2.2 Composite types
- **Struct** (`tipe`): named product type with named fields. Fully implemented
  through native code generation (see §3.5). Structs are compiled to real LLVM
  aggregate types, passed and returned by value, and support nested fields.
- **Enum** (`enum`) **(planned)**: tagged union with pattern matching (`cocok`).
- **Arrays / slices** **(planned)**: `[T; N]` fixed arrays, `iris<T>` slices.

### 2.3 Strings
Design intent: `teks` is an immutable UTF-8 string value. The production layout
is a fat pointer `{ ptr: bita*, len: u64 }` with owned/borrowed variants.

Bootstrap simplification: string literals are emitted as NUL-terminated static
UTF-8 buffers and `teks` is represented as `i8*`. This is sufficient for the
current I/O surface and is called out in `docs/MEMORY_MODEL.md` as interim.

### 2.4 Type inference
Local bindings without an annotation infer their type from the initializer.
Function parameters and return types are always explicit. Integer/float literal
defaults apply only when no other constraint fixes the type.

---

## 3. Declarations

### 3.1 Program header
Every compilation unit begins with a program header naming the unit:
```lime
program Halo
```

### 3.2 The entry point
Exactly one `utama` block per program defines the process entry point:
```lime
utama {
    tampilkan("Halo dunia!")
}
```
`utama` has no parameters and implicitly returns process exit code `0`.
`utama` may contain a trailing `kembalikan <angka>` to set the exit code.

### 3.3 Bindings
```lime
nilai nama = "Lime"        // immutable
nilai umur: angka = 20     // immutable, annotated
ubah skor: i32 = 0         // mutable
skor = skor + 10           // reassignment (only for `ubah`)
```
Assigning to a `nilai` binding is a compile error.

### 3.4 Functions
```lime
fungsi tambah(a: angka, b: angka) -> angka {
    kembalikan a + b
}

fungsi sapa(nama: teks) {          // no `-> T` => returns hampa
    tampilkan(nama)
}
```
All paths through a function with a non-`hampa` return type must return a value
(checked by control-flow analysis).

### 3.5 Structs
Declaration:
```lime
tipe Pemain {
    posisi: Titik
    skor: i32
}
```
Construction uses a struct literal — the type name followed by a brace list of
`field: nilai` pairs (trailing comma allowed). Every field must be supplied
exactly once, with a value assignable to the field's type:
```lime
nilai p = Pemain {
    posisi: Titik { x: 3, y: 4 },
    skor: 100
}
```
Field access reads a field (`p.skor`); nested access chains (`p.posisi.x`).
Field mutation (`p.skor = 150`, `p.posisi.y = 12`) is allowed only when the
root binding is `ubah` (mutable); the whole aggregate is treated as one owned
value, so mutating any field requires the owner to be mutable.

Structs are values: they are copied on binding, passed to functions by value,
and returned by value. Nested structs are supported. Field visibility (`publik`)
is parsed and reserved for cross-module access checking (module system, planned);
within a single module all fields are accessible.

**Struct-literal disambiguation.** A struct literal is not recognised directly
in the condition of `jika`/`selama` or the bounds of `untuk`, because `Nama {`
there would be ambiguous with the block that follows. Wrap it in parentheses to
force a struct literal in those positions.

Representation: each struct becomes an LLVM aggregate type; construction lowers
to `insertvalue`, reads to `extractvalue`, and mutation to a load / nested
`insertvalue` / store. See `docs/MEMORY_MODEL.md`.

---

## 4. Statements

```lime
jika c { ... } lain jika d { ... } lain { ... }

selama kondisi { ... }

untuk i dari 0 sampai 10 { ... }     // half-open: 0,1,...,9
untuk i dari 0 hingga 10 { ... }     // inclusive: 0,1,...,10

berhenti        // break
lanjut       // continue
kembalikan e // return
```
The `untuk` loop variable is an immutable `i64` binding scoped to the loop body.
`sampai` is an exclusive upper bound; `hingga` is inclusive.

---

## 5. Expressions & operators

Precedence, high to low:

| Level | Operators | Assoc |
|---|---|---|
| 1 | `f(...)` call, `.field` access | left |
| 2 | unary `-` `bukan`/`!` | right |
| 3 | `*` `/` `%` | left |
| 4 | `+` `-` | left |
| 5 | `<` `<=` `>` `>=` | left |
| 6 | `==` `!=` | left |
| 7 | `dan` / `&&` | left |
| 8 | `atau` / `||` | left |

`dan`/`atau` short-circuit. Keyword and symbolic spellings are interchangeable
(`dan`≡`&&`, `atau`≡`||`, `bukan`≡`!`). There is **no** implicit numeric
conversion: mixing `i32` and `i64` in an arithmetic expression is a type error;
convert explicitly.

---

## 6. Built-ins

`tampilkan(x)` — print `x` followed by a newline. Accepts `teks`, any integer
type, `f32`/`f64`, `boolean`, and `karakter`. Lowered to a `printf`-family call
against libc through Lime's FFI path (native ABI, not C source).

---

## 7. Modules & FFI **(planned surface, partially wired)**

```lime
gunakan matematika        // standard-library module
impor utilitas            // user module (path-resolved)

ekstern fungsi printf(fmt: teks, ...) -> i32   // C ABI FFI declaration
```
Module resolution, visibility and circular-import detection are specified in
`docs/ARCHITECTURE.md`.

---

## 8. Diagnostics

Every error carries a source span (file, line, column), a stable error code
(`E####`), a message, an underlined snippet, and optional notes/help. See
`docs/ARCHITECTURE.md` §Diagnostics. Diagnostic *text* is in Indonesian to match
the language identity.

---

## 9. Example

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
