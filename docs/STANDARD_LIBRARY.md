# Exotickik Standard Library

The standard library is modular and expands incrementally. The bootstrap ships a
minimal, honest surface; unimplemented modules are listed as planned so nothing
claims to exist before it does.

## Library modules (`gunakan`)

Library modules are written **in Exotickik** (under `std/` in the repository) and
embedded in the compiler; a program pulls one in with `gunakan <Nama>`. Their
public functions then follow the ordinary module-visibility rules. Available
today:

### `Inti` (core numeric helpers)
- `maks(a, b)` / `min(a, b)` — larger / smaller of two `angka`.
- `mutlak(n)` — absolute value.
- `batas(n, bawah, atas)` — clamp `n` into `[bawah, atas]`.
- `tanda(n)` — `-1`, `0`, or `1`.

### `Matematika` (integer math)
- `pangkat(basis, eksp)` — integer power.
- `fpb(a, b)` / `kpk(a, b)` — gcd / lcm.
- `apakah_prima(n)` — primality test (`boolean`).
- `faktorial(n)` — returns `Hasil<angka, teks>`, failing for negative input.

```exotickik
program Hitung
gunakan Matematika
utama { nilai f = faktorial(5) tampilkan(nilai_atau(f, 0)) }   // 120
```

These are real, tested Exotickik functions — not placeholders. Modules that need
language/runtime features still on the roadmap (growable collections, mutable /
owned strings, file and process I/O) are listed under **Planned modules** and are
not shipped until they can be implemented honestly.

## Available now (builtins)

### Core I/O
- `tampilkan(x)` — print a value followed by a newline. Built into the compiler
  and lowered to a libc `printf` call over Exotickik's native-ABI FFI. Accepts
  `teks`, all integer types, `f32`/`f64`, `boolean`, `karakter`, and enums
  (printed as the variant name).

  Formatting: integers print in decimal (signed/unsigned per type), floats with
  a compact `%g` form, `boolean` as `benar`/`salah`, `teks` verbatim. Non-ASCII
  `karakter` printing is a known bootstrap limitation — use `teks` for those.

### Length
- `panjang(x)` — for an array `[T; N]` returns `N` (a compile-time constant); for
  a `teks` returns the UTF-8 **byte** length (via libc `strlen`). Result type
  `angka`.

### Owned heap cell
- `kotak(x)` — allocate a `kotak<T>` on the heap (libc `malloc`) and move `x`
  into it. The box is an owned value: it moves rather than copies and is freed
  (libc `free`) at its owner's drop point. See `docs/MEMORY_MODEL.md`.
- `isi(k)` — read a copy of the contents of a `kotak<T>` (requires `T` to be a
  Copy type). Borrows `k` without consuming it; also accepts a reference to a box
  (`&kotak<T>` / `&ubah kotak<T>`).

### References
- `ambil(x)` — create a shared reference `&T` to the place `x` (a variable,
  field, or array element). Read-only; any number may be live at once. Borrowing
  does not move or copy `x`.
- `ambil_ubah(x)` — create a mutable reference `&ubah T` to `x`; requires `x` to
  be reachable through a `ubah` binding. A mutable reference is exclusive.

  Dereference a reference with `*r` (read) and, for `&ubah`, `*r = e` (write). A
  reference is a native pointer, costs nothing at runtime, and may not outlive or
  escape the scope of the value it points at. See `docs/MEMORY_MODEL.md` for the
  full borrowing rules.

### Error handling
- `sukses(x)` / `gagal(e)` — build a `Hasil<T, R>` success or failure. The
  unconstrained type parameter comes from the expected type in scope.
- `coba r` — propagation operator: yields the success value of `r`, or returns
  the failure early from the enclosing `Hasil<_, R>` function.
- `berhasil(r) -> boolean` — is `r` a success?
- `nilai_atau(r, cadangan) -> T` — the success value, or `cadangan` on failure.
- `gagal_atau(r, cadangan) -> R` — the error value, or `cadangan` on success.

  `Hasil<T, R>` is a value type (a tagged aggregate, no allocation); `T` and `R`
  must be Copy types. See `docs/TYPE_SYSTEM.md` for the full model.

The heap runtime for owned values is libc `malloc`/`free`, reached through Exotickik's
own C-ABI FFI path — no separate allocator library. Growable containers
(`daftar<T>`) will be standard-library types built on `kotak` and this runtime.

### File I/O
- `tulis_file(nama, ambil(buf)) -> boolean` — write a `daftar<bita>` byte buffer
  to the file `nama`, truncating it. Returns `benar` on success.
- `baca_file(nama, ambil_ubah(buf)) -> boolean` — read the whole file `nama` into
  `buf` (replacing its contents, freeing the old buffer). Returns `benar` on
  success, `salah` if the file cannot be opened.

  Built on libc `fopen`/`fread`/`fwrite`/`fclose`; the `FILE*` handle is internal.
  Byte buffers combine with the `Teks` builders below to produce and consume
  files. Richer error values (beyond a boolean) await heap-owned results.

### Text (byte buffers)
- `bita_pada(s, i) -> bita` — byte `i` of a `teks` (bounds-checked). For scanning.
- `cetak_bita(ambil(buf))` — write a `daftar<bita>` byte buffer to output.
- Module `Teks` (`gunakan Teks`), written in Exotickik. A `daftar<bita>` byte
  buffer is Exotickik's owned, growable, mutable string (owned storage, `panjang`
  length, `elemen` indexing, deterministic cleanup on drop); `Teks` adds the
  building and processing operations:
  - Build: `tambah_teks(buf, s)`, `tambah_angka(buf, n)`, `tambah_bita(buf, b)`,
    `baris_baru(buf)` — append into a `&ubah daftar<bita>`.
  - Compare: `sama_teks(&buf, s) -> boolean` (buffer vs `teks`),
    `sama_bita(&a, &b) -> boolean` (buffer vs buffer).
  - Search / match: `cari_bita(&buf, c) -> angka` (index of byte `c`, or `-1`),
    `awalan(&buf, s) -> boolean` (does the buffer start with `s`?).
  - Slice: `potong(&buf, mulai, akhir) -> daftar<bita>` — a new owned buffer with
    bytes `[mulai, akhir)`.

  These cover the string handling a lexer needs (keyword matching, identifier
  extraction, comment scanning). A first-class owned `teks` *value* type and a
  buffer→`teks` conversion remain follow-ups.

### Collections
- `daftar()` — a new empty growable list `daftar<T>` (element type from context).
- `dorong(ambil_ubah(d), x)` — append `x` (grows via `realloc`).
- `elemen(ambil(d), i) -> T` — element `i` (runtime bounds-checked).
- `atur(ambil_ubah(d), i, x)` — overwrite element `i`.
- `panjang(ambil(d)) -> angka` — length.

  `daftar<T>` is Owned (frees its buffer on drop); `T` must be Copy for now. See
  `docs/MEMORY_MODEL.md`.

### Maps (`peta<K, V>`)
- `peta()` — a new empty hash map `peta<K, V>` (type from context).
- `taruh(ambil_ubah(m), k, v)` — insert or overwrite key `k` with value `v`.
- `ambil_peta(ambil(m), k, cadangan) -> V` — value for `k`, or `cadangan` if absent.
- `ada(ambil(m), k) -> boolean` — is key `k` present?
- `panjang(ambil(m)) -> angka` — number of entries.

  `peta<K, V>` is a real open-addressing hash table (polynomial string hash, linear
  probing, automatic growth and rehash at 75% load). It is Owned (frees its two
  buffers on drop). Keys are `teks` for now (string-keyed — the shape symbol
  tables, interning, and module tables need); integer keys are a documented
  follow-up. Values must be Copy. Usable as a function parameter by reference
  (`&peta<K, V>` / `&ubah peta<K, V>`). See `docs/MEMORY_MODEL.md`.

### Concurrency
- `mulai(f, n)` — run `f` (a `fungsi(angka) -> angka`) on a new OS thread with
  argument `n`; returns a thread handle `Utas<angka>`.
- `tunggu(h)` — join the thread `h` and return its result.

  Real `pthread` threads; the argument is passed by value and the result comes
  back through the join, so there is no shared mutable state (see
  `docs/MEMORY_MODEL.md`). POSIX hosts, `angka -> angka` workers for now.

### Strings
- Content comparison with `==` / `!=` (via libc `strcmp`). See
  `docs/LANGUAGE_SPEC.md` §2.3 for the full string model. Concatenation, slicing
  and owned strings are pending the allocator and are not yet available.

### FFI pointers
- `kosong() -> pointer` — the null native pointer. Used to pass NULL to an
  `ekstern` function and, with `==` / `!=`, to null-check a `pointer` returned by
  one. `pointer` is an opaque FFI handle: it cannot be dereferenced, cast, or
  printed (see `docs/LANGUAGE_SPEC.md` §7).

## Planned modules

Following the layout in `task.md`, under `exotickik/std/`:

```
io           buffered readers/writers, formatted output, stdin
string       owned strings, slicing, search, UTF-8 iteration
collections  daftar<T> (vector), peta<K,V> (map), himpunan<T> (set)
filesystem   file open/read/write, paths, directory iteration
process      spawn, environment, exit codes
math         numeric functions, constants
time         clocks, durations
threading    threads, synchronization primitives
networking   TCP/UDP sockets
system       platform info, allocation utilities
```

These require language features still on the roadmap (generics for
`collections`, ownership for buffered I/O, the full `ekstern` FFI surface for
`filesystem`/`networking`). Each module lands with a stable interface first and
grows behind it, per the project's development rules.

## FFI

C libraries are reachable only through Exotickik's `ekstern fungsi` declarations,
which bind to the platform ABI. This is the sole interop path:
`Exotickik → native ABI → C library`. Exotickik never emits or consumes C/C++ source.
