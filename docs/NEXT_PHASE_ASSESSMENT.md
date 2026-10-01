# Phase 16 (Collections) — Design Assessment

Baseline: `main` after the optimizer — 245 tests, 0 warnings. This is the first
self-hosting prerequisite: a growable collection.

## Goal

A real dynamic array `daftar<T>` (list/vector): heap-backed, growable, bounds
checked, owned (freed on drop). Not a fixed `[T; N]` — it grows at runtime.

## Model

`daftar<T>` is an **Owned** (`milik`) value laid out as `{ data: *T, len, cap }`.
It owns its heap buffer and frees it deterministically on drop, exactly like
`kotak<T>`. For this phase `T` must be Copy (`salinan`), so elements need no
per-element drop — the same restriction `Hasil` uses, and enough for the
self-hosting data (`daftar<angka>`, `daftar<teks>`, `daftar<struct-of-copy>`).

## Operations (builtins, reference-based like the borrow model)

- `daftar()` — a new empty list; element type from the expected type in scope.
- `dorong(ambil_ubah(d), x)` — append `x`, growing (via `realloc`, doubling) when
  full.
- `pada(ambil(d), i) -> T` — element `i`, runtime bounds-checked (abort on OOB).
- `atur(ambil_ubah(d), i, x)` — overwrite element `i`, bounds-checked.
- `panjang(ambil(d)) -> angka` — current length (extends the existing `panjang`).

Accessors take references so they never consume the list, consistent with the
reference/borrow rules; the borrow checker already governs the loans.

## Implementation

Reuses the tested Lime-IR → LLVM path rather than hand-written LLVM:

- New `Type::Daftar(T)`, lowered to the aggregate `{ ptr, i64, i64 }`.
- Two new IR ops: `SizeOf { ty }` (byte size of `T`, via the null-GEP/ptrtoint
  trick already used by `kotak`) and `GepElem { base, elem_ty, index }` (a flat
  `getelementptr T, ptr base, i64 index` for buffer indexing).
- `dorong` lowers to real IR: load `len`/`cap` through the `&ubah` reference,
  branch to a grow block that `realloc`s and doubles capacity, then store the
  element and bump `len`. `elemen`/`atur` bounds-check `i` against the runtime `len`
  and load/store through `GepElem`. `daftar()` builds `{ null, 0, 0 }`.
- Ownership: `daftar<T>` is Owned; its drop frees the buffer (`free`). Moves,
  use-after-move, and drops reuse the existing ownership machinery.

## Diagnostics

`E02E0` — collection builtin misuse (wrong arity/receiver, non-Copy element, or a
`daftar()` with no inferable element type).

## Non-goals this phase (documented)

- Owned element types (need per-element drop over the buffer).
- Insert/remove-at, iteration sugar, capacity control — follow-ups.
- Maps/sets — a later collection once hashing/strings are richer.
