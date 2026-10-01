# Phase 17 (Dynamic text + casts) — Design Assessment

Baseline: `main` after collections — 256 tests, 0 warnings. Second self-hosting
prerequisite: manipulating text.

## Goal

Build and scan text dynamically — enough for a compiler to read source and emit
output — without the dangling-pointer problem of a heap-owned `teks` on the
current Copy-string model.

## Approach

Text is a **byte buffer**: `daftar<bita>` (the growable list from Phase 16, over
`u8`). This reuses the owned, bounds-checked, deterministically-freed list
instead of inventing an owned-string type whose borrowed `i8*` could dangle.

Two primitives bridge `teks` and byte buffers:
- `bita_pada(s, i) -> bita` — read byte `i` of a `teks` (bounds-checked via
  `strlen`), for scanning input.
- `cetak_bita(&daftar<bita>)` — write a byte buffer to output (`printf %.*s`).

String *building* is then written **in Lime** as the `Teks` library module
(`tambah_teks`, `tambah_angka`, `tambah_bita`, `baris_baru`), appending into a
`&ubah daftar<bita>`. Those functions forward the mutable list reference through
calls, which the borrow checker already governs.

## Casts (`sebagai`) — the enabling sub-feature

Building text needs integer↔byte conversion (`(48 + d) sebagai bita`), and Lime
had no cast. Added `e sebagai T`: a cast expression binding tighter than binary
operators, type-checked to numeric/`karakter` sources and targets (`E02F0`
otherwise), lowered via the existing `Cast` IR op. The `CastKind` set gained
`sitofp`/`uitofp`/`fptosi`/`fptoui`, and the lowering helper now covers int↔int,
int↔float, float↔float, and int↔`karakter`.

## Implementation

- Lexer keyword `sebagai`; AST `ExprKind::Cast`; a `parse_cast` layer under the
  binary parser; sema `check_cast`; lowering through `cast()`.
- Builtins `bita_pada` / `cetak_bita`, using a flat `GepElem` into the `teks`
  pointer and the list buffer; `Teks` added to the embedded std registry.

## Validation

Casts (`300 sebagai bita`, `3 sebagai f64`, …), reading a string's bytes,
building formatted output (numbers, signs, newlines) and reversing a string via a
byte buffer all run; CLI and sema tests cover the cast rules and the text
builtins; `examples/teks_dinamis.lime` builds a report.

## Non-goals this phase (documented)

- A heap-owned `teks` value (so a built buffer becomes a first-class string) —
  needs owned-string lifetime tracking; byte buffers cover the self-hosting need
  meanwhile.
- String slicing/searching helpers (can be written in `Teks` on top of these).
- Bool casts; `karakter`↔float directly (go via an integer).
