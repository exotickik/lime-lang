# Phase 20 (Benchmark runner `lime bench`) — Design Assessment

Baseline: `main` after the test runner (Phase 19). Fifth self-hosting
prerequisite: a reproducible way to measure performance.

## Goal

A `lime bench <file>` command that measures benchmark functions through the real
backend and reports a stable nanoseconds-per-operation figure, so performance
changes to the compiler can be tracked over time.

## Benchmark function contract

A benchmark is any top-level function that
- is named with the `tolok_` prefix (*tolok ukur* = benchmark),
- takes no parameters and no type parameters,
- returns `angka`.

The `angka` return is required and is the anti-dead-code mechanism (below).
Violations are compile errors pointing at the function. As with `lime test`,
benchmarks are discovered only in the entry file.

## Approach — reuse the test-runner machinery

`lime bench` shares the Phase 19 plumbing: `parse_merge` (parse + merge before
analysis), AST synthesis of a `utama`, then the real
`analyze_modules → lower → optimize_level → build_executable` pipeline, run with
stdout captured.

For each benchmark it synthesizes a scoped block:

```lime
{
    nilai t0: angka = clock()
    ubah jumlah: angka = 0
    ubah i: angka = 0
    selama i < <iters> {
        jumlah = jumlah + tolok_x()
        i = i + 1
    }
    nilai dt: angka = clock() - t0
    tampilkan("##LIME-TOLOK tolok_x")
    tampilkan(dt)
    tampilkan(jumlah)
    fflush(0)
}
```

Each benchmark is its own block scope, so the loop variables (`t0`, `jumlah`,
`i`, `dt`) are reused without collision. The driver parses the three marker lines
(header, elapsed ticks, checksum) back into a table.

## Timing

Timing uses libc `clock()`, injected as `ekstern fungsi clock() -> angka` (an
i64 `clock_t`). POSIX fixes `CLOCKS_PER_SEC` at 1,000,000, so one tick is one
microsecond and `ns/op = dt_ticks * 1000 / iters`. `clock()` measures CPU time,
which is reproducible for CPU-bound microbenchmarks and needs no struct
out-parameter (unlike `clock_gettime`). The reproducibility this buys is visible
in practice: `tolok_pangkat` reports ~24 ns/op whether run at 200k or 1M
iterations.

## Defeating dead-code elimination

Benchmarks are built at the chosen optimization level (default `-O2`) so they
measure optimized code. To stop the optimizer from deleting the measured work,
each call's `angka` result is summed into `jumlah`, which is printed as a
checksum — a genuine data dependency from the loop body to observable output.
The checksum doubles as a correctness signal: `tolok_pangkat` returning `3^15`
over 1,000,000 iterations prints `checksum=14348907000000`, proving the loop ran.
Callers should still keep the measured work non-trivial so it is not folded to a
loop-invariant constant.

## Surviving a crashing benchmark

As in Phase 19, stdout is flushed with `fflush(0)` after each benchmark (null
`FILE*` as an `angka` `0` over the injected `fflush` extern), so a benchmark that
aborts still leaves the prior results intact; it and the ones after it are listed
as `berhenti sebelum selesai` and the command exits non-zero.

## Implementation

- `lime_driver`: `Compiler::bench`, `collect_benches`, `synth_bench_main`,
  `synth_clock_decl`, `report_benches`, and AST builders `mk_name`/`mk_bin`/
  `mk_let` (alongside the Phase 19 builders).
- `lime_cli`: `bench` subcommand with `--iterasi`/`-n`, and help text.

## Tests

- `cli/tests/bench_cmd.rs`: reports ns/op and iteration count, the deterministic
  checksum proves work ran, wrong return type is a compile error, no benchmarks
  is an error, and an aborting benchmark still reports the prior one.
- `examples/tolok_matematika.lime`: three real benchmarks over `Matematika`.

## Honest limitations

- Fixed iteration count (no auto-tuning to a target duration yet); the user
  picks `--iterasi` large enough for stable timing.
- CPU time via `clock()` only; no wall-clock, no per-run variance/stddev, no
  warmup phase, and no memory/allocation accounting.
- One entry file per invocation; no project-wide benchmark discovery.
- Shares the single-process model with `lime test`: a crashing benchmark halts
  the rest.
