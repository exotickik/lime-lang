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
program  modul    utama    fungsi   kembalikan
nilai    ubah
jika     lain     selama   untuk    dari   sampai   hingga
berhenti    lanjut
benar    salah
tipe     enum     sifat    implementasi   cocok
gunakan  impor    ekstern  publik
dan      atau     bukan    coba
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
- **Enum** (`enum`): a fixed set of named variants (see §3.6). Implemented as a
  C-style tagged type; associated data and pattern matching (`cocok`) are the
  next step and the representation is designed to accommodate them.
- **Array** `[T; N]`: fixed-size inline array (see §3.7). Slices (`[T]`) planned.
- **`kotak<T>`**: an owned, heap-allocated cell holding one `T` (see §3.9). This
  is Lime's first **owned (`milik`)** type; it moves rather than copies.
- **`daftar<T>`**: a growable, heap-backed list of `T` (a vector). Owned
  (`milik`): it owns its buffer and frees it on drop. Built with `daftar()`;
  `dorong`/`elemen`/`atur`/`panjang` append, read, write, and measure it. `T`
  must be Copy for now. See §7.2.
- **Function value** `fungsi(A, B) -> R`: a first-class reference to a function.
  A function name used as a value yields a function pointer; it can be stored,
  passed, returned, and called (`f(x)`). It is a bare machine pointer (Copy, no
  allocation). Example:
  ```lime
  fungsi terapkan(f: fungsi(angka) -> angka, x: angka) -> angka { kembalikan f(x) }
  ```

### 2.3 Strings

**Encoding.** `teks` is a UTF-8 string. Source is UTF-8 and string literals
preserve their bytes exactly.

**Value semantics & mutability.** `teks` is an immutable value. There is no
in-place string mutation; string-producing operations yield new values.

**Representation (bootstrap).** A `teks` is a pointer to a NUL-terminated,
immutable, static UTF-8 buffer. String literals are emitted as such buffers and
deduplicated. The production layout is a fat pointer `{ ptr: bita*, len: u64 }`
with owned and borrowed variants; adopting it (and thus allowing embedded NUL
bytes) is scheduled with the memory model.

**Supported operations (implemented):**
- **Comparison** `==` / `!=` compares **contents** (via libc `strcmp`), not
  addresses; ordering operators (`<`, `>`, …) are **not** defined on `teks`.
- **Length** `panjang(s)` returns the number of **bytes** (UTF-8 code units) as
  `angka`, via libc `strlen`. It is a byte count, not a character count —
  `panjang("🍋")` is `4`.
- **Printing** `tampilkan(s)` writes the bytes followed by a newline.
- **Comparison to non-strings** is a type error; there is no implicit conversion.

**Planned (needs the allocator from the memory-model phase):** concatenation,
slicing, byte/char indexing, character iteration, owned/growable strings, and
number↔string conversion. These are intentionally not yet implemented rather
than faked; see `docs/ROADMAP.md`.

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

### 3.6 Enums
An enum declares a fixed set of named variants:
```lime
enum Hasil {
    Berhasil,
    Gagal,
    Menunggu
}
```
A variant is referred to as `Enum.Variant`:
```lime
nilai s = Hasil.Berhasil
```
Enums are value types. Supported operations:
- construction of a variant (`Hasil.Gagal`),
- equality and inequality (`==`, `!=`) between values of the same enum,
- use as a binding, function parameter, return value, and struct field,
- `tampilkan(e)` prints the **variant name** (e.g. `Berhasil`).

Ordering comparisons (`<`, `>`) and arithmetic are not defined on enums, and an
enum value is not interchangeable with an integer — comparing an enum to an
integer is a type error.

Representation: each enum is a distinct type carried as an `i32` tag whose value
is the variant's declaration index (the first variant is `0`). Variant-name
printing lowers to a `select` chain over the tag. Associated data per variant and
`cocok` pattern matching are planned; the tagged representation is chosen so a
payload union can be added later without changing the surface for tag-only enums.

### 3.7 Arrays
A fixed-size array type is written `[T; N]` where `T` is the element type and `N`
is a compile-time length:
```lime
nilai angka: [i32; 4] = [10, 20, 30, 40]
```
An array literal `[e1, e2, ...]` has type `[T; k]` inferred from its elements
(all of which must share the element type). Operations:

- **Indexing** `xs[i]` reads element `i`; `i` is any integer expression.
- **Mutation** `xs[i] = v` writes element `i`, allowed when the array's root
  binding is `ubah`.
- **Length** `panjang(xs)` returns the element count as `angka` (a compile-time
  constant).
- **Iteration** in two forms:
  ```lime
  untuk x pada xs { ... }               // for-each: x is a copy of each element
  untuk i dari 0 sampai panjang(xs) { xs[i] }   // index-based
  ```
- **Bounds checking**: every index (in `xs[i]`, read or write) is checked at
  runtime; an out-of-range index aborts the process (via libc `abort`) rather
  than performing an out-of-bounds memory access. Compile-time-constant indices
  are checked and the branch folds away.

Arrays are value types: assignment, passing to a function, and returning copy the
whole array. Element type checking is fully static — an array of `i32` cannot
hold a `boolean`. Nested aggregates (arrays of structs, structs containing
arrays) are supported.

Representation: `[T; N]` lowers to the LLVM array type `[N x T]`. Element access
uses `getelementptr` + `load`/`store`; array literals build the aggregate with
`insertvalue`. Dynamically sized arrays / slices (`[T]` views with a runtime
length) are planned and will build on this foundation.

### 3.8 Generic functions
A function may declare type parameters in `<...>` after its name:
```lime
fungsi maksimum<T>(a: T, b: T) -> T {
    jika a > b { kembalikan a }
    kembalikan b
}
```
Type parameters may appear anywhere a type is written in the signature or body.
Calls carry no explicit type arguments — each type parameter is inferred from the
argument types (`maksimum(3, 9)` infers `T = angka`; `maksimum(2.5, 1.5)` infers
`T = f64`). Every distinct instantiation is compiled to a separate specialized
function (monomorphization), so generics cost nothing at runtime. A generic body
is checked per instantiation, so operations like `a > b` are validated for each
concrete `T`. See `docs/TYPE_SYSTEM.md` for the full rules and current limits
(generic structs/enums, explicit type arguments, and trait bounds are planned).

### 3.9 Ownership and `kotak<T>`
Lime has no garbage collector. Every type is either **`salinan` (Copy)** — all
the types above — or **`milik` (Owned)**. The category is decided by the
compiler, not annotated. A type is Owned iff it is `kotak<T>` or an aggregate
that transitively contains one.

`kotak<T>` is an owned, heap-allocated cell:
```lime
nilai b = kotak(42)      // allocates a heap i64, owned by b
tampilkan(isi(b))        // isi reads a copy of the contents: 42
```
- `kotak(x)` allocates and moves `x` into the box.
- `isi(k)` reads a copy of the contents (the contents must be a Copy type).

Owned values **move**: binding, passing, returning, or placing one into a
literal transfers ownership and leaves the source moved-out. Using a moved-out
binding is an error:
```lime
nilai b = kotak(5)
nilai c = b              // b moved into c
tampilkan(isi(b))        // error E0280: nilai ini sudah dipindahkan
```
An Owned value is **dropped** (its heap freed, recursively for nested owned
values) when its owner goes out of scope or before an early `kembalikan` /
`berhenti` / `lanjut`. Drops are inserted by the compiler; there is no runtime
reference counting and no garbage collector.

Additional rules: moves must be consistent across `jika` arms (`E0281`); a loop
body may not move a binding declared outside it (`E0282`); Owned bindings are
single-assignment (`E0284`); Owned types may not cross the FFI boundary
(`E0283`). References that borrow a value are §3.10. The full model is in
`docs/MEMORY_MODEL.md`.

### 3.10 References (`&T`, `&ubah T`)
A reference borrows an existing value without owning it. `&T` is a shared
(read-only) reference; `&ubah T` is a mutable one. References are created with
the builtins `ambil` / `ambil_ubah`, read with `*r`, and (for `&ubah`) written
with `*r = e`:
```lime
fungsi naik(x: &ubah angka) {
    *x = *x + 1           // write through a mutable reference
}
utama {
    ubah n = 10
    naik(ambil_ubah(n))   // borrow n mutably for this call
    tampilkan(n)          // 11 — n was mutated in place
}
```
- `ambil(x)` yields `&T`; `ambil_ubah(x)` yields `&ubah T`. The argument must be
  a place — a variable, field, or array element — else `E0290`; `ambil_ubah`
  needs a `ubah` place, else `E0291`.
- Borrowing never moves or copies: `x` stays fully owned and usable afterwards,
  and borrowing a `kotak<T>` does not transfer ownership.
- **Aliasing:** any number of shared references may be live together, but a
  mutable reference is exclusive — while it is live no other reference may be
  taken (`E0295`) and the value may not be changed through its owner (`E0296`).
- **No escape:** a reference may not be returned, stored in a struct/array or a
  `kotak`, or cross FFI (`E0297`/`E0298`). This keeps every reference shorter
  lived than its value, so none can dangle — no lifetime annotations are needed.
- A reference is a native pointer: `*r` lowers to a `load` and `*r = e` to a
  `store`, with no allocation, tag, or reference count.

See `docs/MEMORY_MODEL.md` for the complete borrowing rules and their rationale.

### 3.11 Traits (`sifat` / `implementasi`)
A trait names a set of method signatures; an implementation supplies them for a
type. Dispatch is **static**: a method call compiles to a direct function call,
with no vtable and no runtime cost.
```lime
sifat Luas {
    fungsi luas(diri: &Diri) -> angka
}
implementasi Luas untuk Persegi {
    fungsi luas(diri: &Diri) -> angka { kembalikan diri.sisi * diri.sisi }
}
implementasi Persegi {
    fungsi perbesar(diri: &ubah Diri, k: angka) { diri.sisi = diri.sisi * k }
}
fungsi cetak_luas<T: Luas>(b: &T) { tampilkan(b.luas()) }

utama {
    ubah p = Persegi { sisi: 3 }
    p.perbesar(2)              // inherent method, mutates through &ubah Diri
    tampilkan(p.luas())        // 36
    cetak_luas(ambil(p))       // trait-bounded generic, resolved statically
}
```
- `Diri` is the implementing type; the first parameter's form (`&Diri`,
  `&ubah Diri`, or `Diri`) sets whether the receiver is borrowed, mutably
  borrowed, or moved. A method call `x.m(a)` passes `x` as that receiver under
  the ordinary borrow/move rules.
- `implementasi Sifat untuk Tipe` provides a trait's methods; `implementasi
  Tipe` adds inherent methods. An impl must match the trait's methods exactly.
- A type parameter may be bounded (`<T: Luas>`, or `<T: A + B>`); at each call
  the concrete type must implement the bounds, and the bounded methods then
  resolve to that type's implementation when the function is monomorphized.
- Not yet: dynamic dispatch, default methods, associated types, generic impls.
  See `docs/TYPE_SYSTEM.md` for the full rules and diagnostics.

### 3.12 Error handling (`Hasil<T, R>`, `coba`)
A recoverable result is the built-in generic type `Hasil<T, R>`: either a success
carrying a `T` or a failure carrying an error `R`. Errors are ordinary values —
there is no unwinding and no hidden control flow.
```lime
fungsi bagi(a: angka, b: angka) -> Hasil<angka, teks> {
    jika b == 0 { kembalikan gagal("pembagian dengan nol") }
    kembalikan sukses(a / b)
}
fungsi rata(total: angka, n: angka) -> Hasil<angka, teks> {
    nilai r = coba bagi(total, n)     // unwraps, or returns the failure early
    kembalikan sukses(r)
}
utama {
    nilai r = rata(100, 0)
    jika berhasil(r) { tampilkan(nilai_atau(r, 0)) }
    lain { tampilkan(gagal_atau(r, "?")) }
}
```
- `sukses(x)` / `gagal(e)` build a result; the other type parameter comes from
  the surrounding expected `Hasil` type (return type, annotation, or argument).
- `coba e` is the propagation operator: it yields the success value, or returns
  the failure from the enclosing function (which must return `Hasil<_, R>` with
  the same error type `R`). Propagation is visible early-return, not an exception.
- `berhasil(r)`, `nilai_atau(r, cadangan)`, and `gagal_atau(r, cadangan)` inspect
  a result at a boundary without assuming a branch.
- For now `T` and `R` must be Copy types (`teks`, numbers, Copy structs …); an
  Owned payload is a diagnostic. `Hasil` is a value — it allocates nothing and
  lowers to a tagged aggregate. See `docs/TYPE_SYSTEM.md`.

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
| 2 | unary `-` `bukan`/`!`, deref `*`, `coba` | right |
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

`panjang(x)` — length of an array or `teks`. `kotak(x)` / `isi(k)` — build and
read an owned heap cell (§3.9). `ambil(x)` / `ambil_ubah(x)` — borrow a place as
a shared or mutable reference (§3.10). `sukses(x)` / `gagal(e)` build a
`Hasil<T, R>`, and `berhasil(r)` / `nilai_atau(r, d)` / `gagal_atau(r, d)`
inspect one (§3.12). `mulai(f, n)` / `tunggu(h)` spawn and join an OS thread
(§7.1). `daftar()` builds a list and `dorong`/`elemen`/`atur` (plus `panjang`)
operate on it (§7.2). These names are reserved builtins and cannot be redefined.

### 7.2 Dynamic lists (`daftar<T>`)
`daftar<T>` is a growable, heap-backed list. It is Owned: it owns its buffer and
frees it on drop, and moves rather than copies. Accessors take references, so
they never consume the list:
```lime
ubah d: daftar<angka> = daftar()
dorong(ambil_ubah(d), 10)          // append (grows the buffer as needed)
dorong(ambil_ubah(d), 20)
tampilkan(panjang(ambil(d)))        // 2
tampilkan(elemen(ambil(d), 0))      // 10  (bounds-checked)
atur(ambil_ubah(d), 1, 99)          // overwrite index 1
```
- `daftar()` — a new empty list; element type from the expected type in scope.
- `dorong(&ubah d, x)`, `atur(&ubah d, i, x)`, `elemen(&d, i) -> T`,
  `panjang(&d) -> angka`. Indices are runtime bounds-checked (abort on OOB).
- `T` must be a Copy type for now; owned element types are a follow-up.

### 7.1 Concurrency (`mulai` / `tunggu`)
`mulai(f, n)` runs the function `f` (a `fungsi(angka) -> angka`) on a new OS
thread with argument `n`, returning a thread handle `Utas<angka>`. `tunggu(h)`
joins the thread and returns its result:
```lime
fungsi kerja(n: angka) -> angka { kembalikan n * n }
utama {
    nilai a = mulai(kerja, 10)
    nilai b = mulai(kerja, 20)
    tampilkan(tunggu(a) + tunggu(b))     // runs kerja on two threads: 500
}
```
Threads are real OS threads (via `pthread`). The argument is passed by value
(a Copy `angka`) and the result is returned through the join, so there is no
shared mutable state and therefore no data races — the safety comes from Lime's
ownership rules, not a lock. This first cut supports `angka -> angka` workers on
POSIX hosts; richer payloads, channels, and Windows threads are documented
follow-ups (`docs/MEMORY_MODEL.md`).

---

## 7. Modules & FFI

A program can span several files. A **library file** begins with `modul <Nama>`;
the **entry file** begins with `program <Nama>` and holds `utama`. `impor <Nama>`
loads `<Nama>.lime` from the importing file's directory:

```lime
// file Matematika.lime
modul Matematika
publik fungsi tambah(a: angka, b: angka) -> angka { kembalikan a + b }
fungsi rahasia() -> angka { kembalikan 0 }        // module-private

// file main.lime
program Aplikasi
impor Matematika
utama { tampilkan(tambah(2, 3)) }                 // 5
```

- `impor` is transitive and cycle-safe; a missing module file is a diagnostic.
- Only `publik` items (functions, structs, enums, traits) cross a module
  boundary, and only into a module that `impor`s the defining one; otherwise
  `E02C0`. `utama` is only allowed in the entry `program` file.
- All modules currently share one flat name space (two modules cannot define the
  same top-level name). Per-module namespaces and qualified names (`Modul.item`)
  are a documented follow-up. `gunakan` is reserved for future standard-library
  modules.

```lime
ekstern fungsi printf(fmt: teks, ...) -> i32   // C ABI FFI declaration
```
Module resolution and visibility are specified in `docs/ARCHITECTURE.md`.

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
