# Phase 18 (File I/O) — Design Assessment

Baseline: `main` after dynamic text + casts (Phase 17). Third self-hosting
prerequisite: a compiler must read a source file and write an output file.

## Goal

Whole-file read and write from Lime, reusing the byte-buffer representation so
reading a file, building text, and writing it out share one type.

## Approach

Files move through `daftar<bita>` byte buffers — the same growable, owned,
bounds-checked list used for dynamic text (Phase 16/17). Two builtins:

- `tulis_file(nama: teks, ambil(buf: daftar<bita>)) -> boolean` — write the whole
  buffer to `nama`, truncating it. Takes a shared reference (read-only).
- `baca_file(nama: teks, ambil_ubah(buf: daftar<bita>)) -> boolean` — read the
  whole file into `buf`, replacing (and freeing) its previous contents. Takes a
  mutable reference. Returns `salah` when the file cannot be opened.

### Why `boolean` and not `Hasil<daftar<bita>, R>`

`Hasil<T, R>` is a Copy value type; its payloads must be Copy, and `daftar<bita>`
is Owned. A read therefore cannot *return* the buffer inside a `Hasil`. The data
is delivered through the caller's mutable buffer (an out-parameter) and the call
yields a `boolean` for success. A richer error type is a follow-up tied to
heap-owned result payloads.

### FILE* across the native ABI

Lowering uses libc `fopen`/`fseek`/`ftell`/`fread`/`fwrite`/`fclose` over the
existing native-ABI FFI. The `FILE*` handle is carried as an opaque `i64` value
(pointer-width on the supported targets), avoiding any `ptrtoint`/`inttoptr`
dance in the emitted IR: the declarations simply type the handle as `i64`. A
`needs_file` module flag gates the six `extern` declarations in codegen so
programs that never touch files emit none of them.

### Read sequence

`fopen(nama, "rb")`; on a null handle, store `salah` and skip. Otherwise
`fseek(END)` + `ftell` to size the file, `malloc` a buffer, `fseek(SET)`,
`fread`, `fclose`. The old `daftar` buffer (field 0) is `free`d and the three
list fields (`ptr`, `len`, `cap`) are overwritten to adopt the freshly read
bytes; `salah`/`benar` is produced via an alloca result slot stored in each
branch and loaded after the merge (the backend has no phi nodes).

### Write sequence

`fopen(nama, "wb")`; null → `salah`. Otherwise `fwrite(ptr, 1, len, f)` from the
buffer's fields, `fclose`, `benar`.

## Implementation

- `Builtin::BacaFile` / `Builtin::TulisFile` (`lime_sema` results).
- `check.rs`: registered in globals + `is_builtin_name`; `check_call` arms type
  the path as `teks`, require `&ubah daftar<bita>` (read) / `&daftar<bita>`
  (write), and return `Type::Bool`.
- `lower.rs`: `lower_baca_file` / `lower_tulis_file` + a `call_i64` helper,
  using the daftar field-pointer helpers from Phase 16.
- `emit.rs`: `needs_file` gate emitting `fopen`/`fseek`/`ftell`/`fread`/`fwrite`/
  `fclose` declarations with the `i64` FILE* convention.

## Tests

- `cli/tests/fileio.rs`: write-then-read round-trip (asserts on-disk content and
  printed bytes), reading a missing file returns `salah`, byte-count of a read
  file.
- `lime_sema`: `file_io_ok`, `baca_file_needs_mutable_buffer` (rejects a shared
  reference where a mutable one is required).
- `examples/berkas.lime`: builds a report with `Teks`, writes it, reads it back,
  prints its length and contents.

## Honest limitations

- Whole-file only; no streaming, seeking from Lime, append mode, or partial
  reads/writes yet.
- POSIX/libc hosts. Windows file I/O rides the same libc path but is unverified
  in the current Linux-only container.
- No path manipulation, directory iteration, or `stat`-style metadata.
- Error reporting is a `boolean`; distinguishing *why* a read failed awaits
  heap-owned `Hasil` payloads.
