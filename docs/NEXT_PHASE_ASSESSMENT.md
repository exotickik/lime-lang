# Phase 27 (`peta<K, V>` hash map) — Design Assessment

Baseline: `main` after the Lime → Exotickik rebrand (commit `7b23bc7`). The
self-hosting gap analysis named a map as the most load-bearing missing container
(symbol tables, interning, module tables). This phase delivers a real hash map.

## Goal

A genuine, growable hash map `peta<K, V>` — not a fixed-key stub and not an
O(n) association list — integrated with the ownership model, references, modules,
and native codegen, with the operations a compiler's symbol tables need.

## Design

- **Type**: `Type::Peta { key, val }`, resolved from `peta<K, V>`. Owned
  (`milik`), so it moves and is dropped deterministically; `V` must be Copy.
- **Keys**: `teks` for now. This is exactly the shape symbol tables, interning,
  and module tables take (string → info); integer keys are a documented
  follow-up. The `peta<K, V>` type stays general; the builtins currently require
  `K == teks`.
- **Representation**: two parallel heap arrays plus two counts —
  `{ ptr keys, ptr values, i64 len, i64 cap }`. A NULL entry in `keys` marks an
  empty bucket, so no separate state array is needed. Choosing parallel arrays
  (rather than one array of `{state, K, V}` entries) avoids synthesizing an entry
  struct type and reuses the existing `GepElem`/`SizeOf` element machinery.
- **Hashing**: a polynomial string hash (`h = h*31 + byte`) over the key's bytes,
  computed inline. Bucket index is `hash urem cap` (unsigned remainder, so `cap`
  need not be a power of two and negative wraparound is impossible).
- **Collisions**: open addressing with linear probing. One probe routine serves
  lookup, insert, and rehash: it returns the matching bucket or the first empty
  one.
- **Growth**: when `(len+1)*4 > cap*3` (75% load, which also covers the `cap==0`
  first insert), capacity doubles (from 8) and every entry is rehashed into fresh
  buffers; the old buffers are freed. Keys buffer is `calloc`-zeroed so empty
  buckets read as NULL.
- **Empty-map safety**: `ambil_peta`/`ada` guard `cap == 0` before probing, so a
  never-inserted map never does `urem 0`.

## Operations

- `peta() -> peta<K, V>` — empty (no allocation until first insert).
- `taruh(ambil_ubah(m), k, v)` — insert or overwrite; grows first if needed; only
  bumps `len` when the key is new (via a `select`, no branch).
- `ambil_peta(ambil(m), k, cadangan) -> V` — value or fallback.
- `ada(ambil(m), k) -> boolean` — membership.
- `panjang(ambil(m)) -> angka` — entry count (the shared `panjang` builtin now
  accepts a `&peta`).

## Ownership and cleanup

`peta<K, V>` is Owned in both `is_copy` paths, so it participates in moves,
use-after-move checking, and deterministic drop exactly like `daftar<T>`.
`emit_drop_value` frees the keys and values buffers at the owner's drop point;
the stored `teks` keys are not owned (they point at existing strings), so only the
two arrays are freed — no double free. References across FFI and owned payloads
across FFI remain rejected.

## Implementation

- `exotickik_types`: `Type::Peta`, display.
- `exotickik_sema`: `peta`/`taruh`/`ambil_peta`/`ada` builtins, `ref_peta`
  helper, `check_peta_builtin` (key must be `teks`, value Copy), `panjang`
  extended, and `Peta` arms in resolve/`is_copy`/`unify`/`subst`/
  `type_to_type_expr_kind`/ownership.
- `exotickik_ir`: `peta_parts`, `hash_teks`, `peta_probe`, `peta_ensure`
  (grow + rehash), `lower_taruh`/`lower_ambil_peta`/`lower_ada`, `panjang` and
  drop arms — all built from existing IR primitives (malloc/calloc/free, GepElem,
  SizeOf, strcmp, Select, loops via blocks).
- `exotickik_codegen`: `ty_str`/`MakeStruct` for the 4-field layout, and a
  `calloc` runtime declaration.

## Tests

- `cli/tests/maps.rs`: put/get/overwrite/len, absent→default, `ada`, empty-map
  safety, growth preserving all ten entries across a rehash, use by reference,
  and three negative cases (non-`teks` key, owned value, move).
- `exotickik_sema`: `peta_ok`, non-`teks` key, owned value, move semantics.
- `examples/peta.exo`: a word-frequency counter.

All 313 prior tests still pass; 0 warnings; repo passes `exotickik fmt --check`.

## Honest limitations

- `teks` keys only (integer/enum keys and a `himpunan<T>` set are follow-ups).
- Values must be Copy (owned values await owned-payload support).
- No deletion yet (no tombstones); scopes are dropped by discarding the map.
- No iteration-over-entries builtin yet (lookup/insert/membership/count only).
