# Phase 14 (Standard Library) — Design Assessment

Baseline: `main` at the Modules checkpoint — 221 tests, 0 warnings.

## Goal

A real standard library, written **in Lime** wherever practical, so it also
serves the eventual self-hosting path. No placeholder APIs: ship only functions
the language and runtime can implement honestly today.

## What Lime can support today

Numbers, `teks` (immutable strings), fixed arrays, `kotak<T>`, structs, enums,
generics, traits, references, `Hasil<T, R>`, modules, and libc FFI. That is
enough for **numeric / integer-math** utilities and result-returning helpers, all
expressible in Lime. Growable collections, mutable/owned strings, and file /
process I/O need runtime features still on the roadmap and are deferred (listed
as planned, not stubbed).

## Delivery mechanism

Standard modules live under `std/` as ordinary `modul` files and are **embedded
into the compiler** with `include_str!` (a small registry in `lime_driver`). A
program pulls one in with `gunakan <Nama>`, which the driver resolves against the
embedded registry exactly as `impor` resolves a sibling file — same merge, same
module metadata, same visibility rules. This keeps the library always available
(no search path), and, because it is plain Lime, it is compiled and checked by
the same pipeline as user code.

## Modules shipped this phase

- **`Inti`** — `maks`, `min`, `mutlak`, `batas` (clamp), `tanda` (sign).
- **`Matematika`** — `pangkat` (integer power), `fpb`/`kpk` (gcd/lcm),
  `apakah_prima`, and `faktorial` returning `Hasil<angka, teks>` (exercising the
  error model inside the library).

## Validation

Programs `gunakan` the modules and run natively; visibility and the flat
namespace behave as for user modules. CLI tests cover each function and the
error paths (unknown library module; using a library function without `gunakan`).
An example (`examples/pustaka.lime`) runs.

## Non-goals this phase (documented)

- Collections (`daftar<T>` vector, maps) — need a dynamic-allocation API beyond
  the single-cell `kotak<T>`.
- String module (concat/slice/owned strings) — needs the owned-string runtime.
- io / filesystem / process / time / net — need the FFI surface and runtime for
  each, taken up in later roadmap phases.
