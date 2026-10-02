# Phase 28 (Dynamic string operations) — Design Assessment

Baseline: `main` after `peta<K, V>` (Phase 27). The next self-hosting prerequisite
is owned/dynamic strings. This phase delivers the string-processing operations a
self-hosted lexer needs, on the string type Exotickik already has.

## What already existed

Exotickik's owned, growable, mutable string is the `daftar<bita>` byte buffer:
owned heap storage, `panjang` for length, `elemen` for indexing, `dorong` to
append, and deterministic cleanup on drop (Phase 16/17). The `Teks` module —
written in Exotickik — already built text into a buffer (`tambah_teks`,
`tambah_angka`, `tambah_bita`, `baris_baru`). So "owned storage, length,
indexing, concatenation, deterministic cleanup" were present.

The genuine gaps for writing a lexer were **comparison, search, prefix-match and
slicing** of these buffers.

## What was added

Five real functions in the `Teks` stdlib module, written in Exotickik on the
existing primitives (no compiler change):

- `sama_teks(&buf, s) -> boolean` — buffer equals a `teks`.
- `sama_bita(&a, &b) -> boolean` — buffer equals buffer.
- `awalan(&buf, s) -> boolean` — buffer starts with a `teks` (prefix match).
- `cari_bita(&buf, c) -> angka` — index of the first byte `c`, or `-1`.
- `potong(&buf, mulai, akhir) -> daftar<bita>` — a new **owned** buffer holding
  bytes `[mulai, akhir)` (slicing that copies).

These are exactly the operations a lexer uses: `awalan`/`sama_teks` for keyword
matching, `potong` for identifier/number extraction, `cari_bita` for scanning to
a delimiter or comment end. `potong` returning an owned `daftar<bita>` composes
with the ownership model (it is moved out and freed at its owner's drop).

## Why stdlib Exotickik, not a new primitive

Writing these in Exotickik (rather than as compiler builtins) is itself a
self-hosting datapoint: the language is expressive enough to implement its own
string library. It also keeps the compiler surface small and the implementations
auditable. The one operation that genuinely needs a compiler primitive — a
buffer→owned-`teks` *value* conversion — is deferred with the first-class owned
`teks` type, because returning a `teks` that borrows into a buffer would dangle
without the owned-string type to hold it.

## Tests

- `cli/tests/stdlib.rs`: `teks_sama_and_awalan`, `teks_cari_and_potong`,
  `teks_sama_bita`.
- `examples/kata.exo`: a lexer-flavored demo — slice the first word out of a
  line, match it against a keyword, and prefix-check the line.

All 326 prior tests still pass (now 329); 0 warnings; repo passes
`exotickik fmt --check`.

## Honest limitations

- These operate on `daftar<bita>` buffers. A first-class owned `teks` value type
  and a buffer→`teks` conversion are still open (see `docs/SELF_HOSTING.md`).
- No Unicode-aware iteration (byte-level only), no case folding, no regex.
- `potong` copies; there is no borrowed slice view (that needs lifetimes).
- Next: `Hasil<T, R>` with owned payloads so diagnostics can carry owned strings.
