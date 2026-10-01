# Phase 19 (Test runner `lime test`) — Design Assessment

Baseline: `main` after file I/O (Phase 18). Fourth self-hosting prerequisite: a
compiler project needs to run its own tests.

## Goal

A `lime test <file>` command that discovers test functions, runs them through the
real backend, and reports pass/fail with a non-zero exit on failure — usable both
by hand and from CI.

## Test function contract

A test is any top-level function that
- is named with the `uji_` prefix (`uji` = *test*),
- takes no parameters,
- has no type parameters,
- returns `boolean` (`benar` = passed).

Violations of the last three are compile errors (`E_UNSUPPORTED`) pointing at the
offending function, so a malformed test never silently disappears. Tests are
discovered only in the **entry file** (the one passed to `lime test`), not in its
imported modules, so a module's own `uji_*` helpers are not run transitively and
cross-module visibility (`publik`) never enters into it.

## Approach — synthesize the harness, reuse the whole pipeline

`lime test` does not invent a second execution path. It:

1. Parses and merges the program exactly as `build`/`run` do (`parse_merge`,
   factored out of the former `frontend`), **before** semantic analysis.
2. Walks the merged AST for valid `uji_*` functions in the entry file.
3. Rejects a file that already has a `utama` block via the normal
   single-main check — the runner owns `utama`.
4. Synthesizes a `utama` as ordinary AST, minting fresh `NodeId`s from the
   parser's node counter (threaded out of `parse_merge`): for each test,
   `jika uji_x() { tampilkan("##LIME-UJI LULUS x") } lain { tampilkan("##LIME-UJI GAGAL x") }`.
5. Runs the real `analyze_modules` → `lower` → `optimize_level` →
   `build_executable` on the augmented program, then executes it capturing
   stdout.
6. Parses the marker lines and prints a human summary, exiting non-zero on any
   failure.

Because the harness is real Lime AST fed through the real sema and codegen, the
tests exercise the actual compiler — there is no mock interpreter.

## Surviving a crashing test

The test executable's stdout is a pipe, so libc fully buffers it; a test that
`abort()`s (e.g. a bounds-check failure) would otherwise lose the buffered
results of every test that already passed. The harness therefore calls
`fflush(0)` after each test's marker. `fflush(NULL)` flushes all open streams;
the null `FILE*` is passed as an `angka` `0` over an injected
`ekstern fungsi fflush(aliran: angka) -> angka`, reusing the FILE*-as-i64 ABI
convention from Phase 18 (an integer and a pointer argument share registers on
the supported ABIs). A test that aborts is then reported as
`berhenti sebelum selesai`; tests after it that never ran are reported the same.
The extern is injected only when the program does not already declare `fflush`.

## Implementation

- `lime_driver`: `parse_merge` (shared parse+merge), `Compiler::test`,
  `collect_tests`, `synth_test_main`, `synth_fflush_decl`, `report_tests`, and
  small AST builders (`mk_expr`/`mk_call`/`mk_print`).
- `lime_cli`: `test` subcommand and help text.

## Tests

- `cli/tests/test_cmd.rs`: all-pass (exit 0), mixed pass/fail (exit 1), wrong
  return type (compile error), parameters (compile error), no tests (error),
  and a runtime abort that still reports the prior passing test.
- `examples/uji_matematika.lime`: five real tests over the `Matematika` module,
  run with `lime test`.

## Honest limitations

- One entry file per invocation; no directory/glob discovery or a project-wide
  `lime test` across all files yet (awaits the package system).
- Tests report a single `boolean`; there is no assertion API with messages,
  `expected vs actual` diffs, or per-assertion locations yet.
- No test filtering, parallelism, timing, or setup/teardown.
- A crashing test halts the single shared executable; per-test process isolation
  is a follow-up.
