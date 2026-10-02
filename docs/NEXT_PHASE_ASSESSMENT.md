# Phase 29 (Owned `Hasil<T, R>` payloads) — Design Assessment

Baseline: `main` after dynamic string operations (Phase 28). The next blocker was
`Hasil<T, R>` rejecting owned payloads (`E02B3` at construction). This phase makes
`Hasil` a real ownership-safe result type.

## Goal

`Hasil<T, R>` may carry Owned payloads — `daftar<T>`, `peta<K, V>`, `kotak<T>`,
owned byte buffers, and owned combinations — with correct moves, tag-conditional
destruction, and `coba` propagation, and no leaks or double-frees.

## Representation (reused, not duplicated)

`Hasil` keeps its existing tagged aggregate `{ i32 tag, T, R }` (tag 0 = `sukses`,
1 = `gagal`). Both payload fields exist in the layout, but only the one named by
the tag is logically alive; the other is `undef` and never read. No second
tagged-union system was introduced.

## Ownership model

- `Hasil<T, R>` is Copy iff both `T` and `R` are Copy (unchanged `is_copy`), so a
  `Hasil` with any owned payload is automatically **Owned**: it moves, is tracked
  for use-after-move, and is dropped at scope end / early return like any owned
  value. No new classification code was needed — the restriction was the only
  thing standing in the way.
- **Construction** (`sukses`/`gagal`): the Copy-only check was removed from
  construction, so owned payloads are accepted. The payload argument is moved in
  (the ownership checker already treats non-borrow call args as consuming). The
  inactive field is filled with a typed `undef` (`undef_of` extended to
  `daftar`/`peta`/`kotak`), which is never dropped.
- **Tag-conditional drop**: `emit_drop_value` gained a `Hasil` arm. If both
  payloads are Copy it is a no-op (unchanged). Otherwise it loads the tag and
  branches: on tag 0 it extracts and drops the `ok` payload (if owned), on tag 1
  the `err` payload (if owned), recursing into nested owned values. The inactive
  (`undef`) field is never freed, so there is no double free.
- **`coba`**: now **consumes** its operand in the ownership checker
  (`Try => visit(inner, true)`). The success path extracts and moves out the `ok`
  payload; the failure path extracts `err` and moves it into the early-returned
  `Hasil`. Because the source is consumed (and its inactive field is `undef`), it
  is not dropped afterward — no leak, no double free, no use-after-move.

## Inspection builtins

`berhasil`/`nilai_atau`/`gagal_atau` take the `Hasil` by value and would have to
consume-and-conditionally-drop an owned payload, which is exactly what `coba` (and
future `cocok` on `Hasil`) express cleanly. Rather than silently copy or leak,
these builtins now **require Copy payloads** (`E02B3`, with help pointing to
`coba`). Owned results are consumed with `coba`, moved, or dropped. This keeps the
ownership model honest and the API small; owned-payload destructuring arrives with
pattern matching on sum types.

## Verification

- `cli/tests/hasil_owned.rs`: `coba` success extraction, `coba` error propagation,
  dropping unused owned success/error results, owned error payload drop, nested
  owned payload (`peta` in `Hasil`), use-after-move rejection, and Copy-payload
  inspection still working.
- `exotickik_sema`: owned construction allowed, owned inspection rejected,
  owned use-after-move; the old "owned rejected at construction" test was updated
  to assert construction now succeeds.
- `cli/tests/errors.rs`: owned construction ok, owned inspection error.
- `examples/hasil_milik.exo`: load a source file into owned bytes, return
  `Hasil<daftar<bita>, teks>`, and size it via `coba` — a compiler-relevant case.
- **Valgrind**: the owned-`Hasil` example and the runtime scratch programs report
  `0 bytes in use at exit` and `0 errors` (no leaks, no double/invalid frees),
  including the owned-error-variant drop path.

All prior tests pass; 0 warnings; repo passes `exotickik fmt --check`.

## Honest limitations

- Inspection builtins are Copy-only; owned-payload inspection awaits `cocok` on
  sum types (enum associated data).
- `coba` requires the enclosing function's error type to match (unchanged); there
  is no error-type conversion.
- A first-class owned `teks` value type is still separate from `daftar<bita>`.

## Next

Generic structs, then generic enums, then enum associated data + `cocok` payload
binding (which also unblocks owned-`Hasil` inspection), then porting compiler
front-end components into Exotickik.
