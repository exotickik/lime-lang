# Lime Standard Library

The standard library is modular and expands incrementally. The bootstrap ships a
minimal, honest surface; unimplemented modules are listed as planned so nothing
claims to exist before it does.

## Available now

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
