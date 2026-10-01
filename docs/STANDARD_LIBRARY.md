# Lime Standard Library

The standard library is modular and expands incrementally. The bootstrap ships a
minimal, honest surface; unimplemented modules are listed as planned so nothing
claims to exist before it does.

## Library modules (`gunakan`)

Library modules are written **in Lime** (under `std/` in the repository) and
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

```lime
program Hitung
gunakan Matematika
utama { nilai f = faktorial(5) tampilkan(nilai_atau(f, 0)) }   // 120
```

These are real, tested Lime functions — not placeholders. Modules that need
language/runtime features still on the roadmap (growable collections, mutable /
owned strings, file and process I/O) are listed under **Planned modules** and are
not shipped until they can be implemented honestly.

## Available now (builtins)

### Core I/O
- `tampilkan(x)` — print a value followed by a newline. Built into the compiler
  and lowered to a libc `printf` call over Lime's native-ABI FFI. Accepts
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

The heap runtime for owned values is libc `malloc`/`free`, reached through Lime's
own C-ABI FFI path — no separate allocator library. Growable containers
(`daftar<T>`) will be standard-library types built on `kotak` and this runtime.

### Collections
- `daftar()` — a new empty growable list `daftar<T>` (element type from context).
- `dorong(ambil_ubah(d), x)` — append `x` (grows via `realloc`).
- `elemen(ambil(d), i) -> T` — element `i` (runtime bounds-checked).
- `atur(ambil_ubah(d), i, x)` — overwrite element `i`.
- `panjang(ambil(d)) -> angka` — length.

  `daftar<T>` is Owned (frees its buffer on drop); `T` must be Copy for now. See
  `docs/MEMORY_MODEL.md`.

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

## Planned modules

Following the layout in `task.md`, under `lime/std/`:

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

C libraries are reachable only through Lime's `ekstern fungsi` declarations,
which bind to the platform ABI. This is the sole interop path:
`Lime → native ABI → C library`. Lime never emits or consumes C/C++ source.
