# Exotickik Toolchain

## The `exotickik` command

```
exotickik build <file>     kompilasi ke executable native
exotickik run <file>       kompilasi lalu jalankan
exotickik check <file>     periksa sintaks dan tipe tanpa kompilasi
exotickik test <file>      jalankan fungsi uji ('uji_*') di file
exotickik fmt [path...]    format kode (--write menimpa, --check periksa)
exotickik emit-ir <file>   cetak LLVM IR yang dihasilkan
exotickik init [nama]        buat proyek Exotickik baru (exotickik.toml + src/main.exo)
exotickik clean              hapus artefak build (.exotickik-build)
exotickik version            tampilkan versi
exotickik help               bantuan
```

Common options for `build` / `run` / `check`:

| Option | Meaning |
|---|---|
| `-o, --output <file>` | output executable name |
| `--target <name>` | `host` (default), `linux-x64`, `windows-x64` |
| `-O0 -O1 -O2 -O3` | optimization level passed to the backend (default `-O2`) |
| `-- <args...>` | arguments forwarded to the program under `exotickik run` |

## Testing (`exotickik test`)

`exotickik test <file>` discovers and runs the test functions in a file. A test
function is any top-level function whose name begins with `uji_`, takes no
parameters, and returns `boolean` — `benar` means the test passed:

```exotickik
program UjiMatematika
gunakan Matematika

fungsi uji_pangkat_dua() -> boolean {
    kembalikan pangkat(2, 10) == 1024
}
```

```
$ exotickik test examples/uji_matematika.exo
menjalankan 5 uji: examples/uji_matematika.exo

  ok     uji_pangkat_dua
  ok     uji_fpb
  ...

hasil: 5 lulus, 0 gagal
```

The test file must not contain a `utama` block — the runner synthesizes one that
calls each `uji_*` function and reports its result. Tests are compiled and run as
one native executable (so they exercise the real backend), with output flushed
after each test so a crash in one test still reports the ones that ran before it;
such a test is listed as `berhenti sebelum selesai`. `exotickik test` exits non-zero
if any test fails or aborts. A `uji_*` function with parameters, type parameters,
or a non-`boolean` return type is a compile error.

## Formatting (`exotickik fmt`)

`exotickik fmt` normalizes Exotickik source. It accepts files, directories (recursed for
`*.exo`, skipping `target`, `.exotickik-build`, and `.git`), or — with no path — the
project: the directory of the nearest `exotickik.toml`.

- `exotickik fmt <file>` — print the formatted file to stdout (single file only).
- `exotickik fmt --write` (`-w`) — rewrite files in place; only changed files are
  touched, and each is reported.
- `exotickik fmt --check` (`-c`) — do not write; list files that are not formatted and
  exit non-zero if any are. This is the CI mode.

```
$ exotickik fmt --check          # whole project
perlu format: src/main.exo
1 file perlu diformat
$ exotickik fmt --write src tests # two directories, rewrite in place
```

Formatting is idempotent (`fmt` of `fmt` output is a no-op) and preserves
semantics; it never changes diagnostics or behavior.

## Benchmarking (`exotickik bench`)

`exotickik bench <file>` measures benchmark functions. A benchmark is any top-level
function whose name begins with `tolok_`, takes no parameters, and returns
`angka` — the return value is summed into a checksum so the optimizer cannot
delete the work being measured:

```exotickik
program TolokMatematika
gunakan Matematika

fungsi tolok_pangkat() -> angka {
    kembalikan pangkat(3, 15)
}
```

```
$ exotickik bench examples/tolok_matematika.exo --iterasi 100000
menjalankan 3 tolok ukur: examples/tolok_matematika.exo (100000 iterasi)

  tolok_pangkat                 2.434 ms total         24.340 ns/op   checksum=1434890700000
  ...
```

Each benchmark runs a fixed number of iterations (`--iterasi N`, default
1,000,000), timed with libc `clock()` (CPU time; POSIX fixes `CLOCKS_PER_SEC` at
1,000,000, so one tick is one microsecond). The harness reports total time,
nanoseconds per operation, and the checksum. Benchmarks are compiled at the
chosen optimization level (default `-O2`), so they measure optimized code; keep
the measured work non-trivial so it is not folded to a constant. Benchmark
functions should not print to stdout (the runner reads timing from it). Output is
flushed after each benchmark, so a benchmark that aborts is reported as
`berhenti sebelum selesai` and `exotickik bench` exits non-zero.

## Build pipeline

`exotickik build` runs: lex → parse → semantic analysis → Exotickik IR lowering →
Exotickik IR optimization → LLVM IR emission → `llc` (object) → linker (executable).

Intermediate files (`<program>.ll`, `<program>.o`) are written under a
`.exotickik-build/` directory next to the output. The backend locates tools in this
order (environment override first):

| Purpose | Env override | Fallback candidates |
|---|---|---|
| IR → object | `EXOTICKIK_LLC` | `llc`, `llc-18`, `llc-17` |
| link (native ABI) | `EXOTICKIK_CC` | `cc`, `clang`, `gcc` |
| link (Windows) | `EXOTICKIK_LINKER` | `lld-link` |

No internet access is required for compilation.

## Projects and the manifest (`exotickik.toml`)

`exotickik init [nama]` scaffolds a project: a `exotickik.toml` manifest and
`src/main.exo`.

```toml
[paket]
nama = "halo"
versi = "0.1.0"

[build]
entri = "src/main.exo"   # entry file (default: src/main.exo)
target = "host"           # host | linux-x64 | windows-x64
optimasi = 2              # 0..3 (default 2)
```

When `build`, `run`, `check`, `test`, or `bench` is invoked **without a file
argument**, Exotickik searches the current directory and its ancestors for a
`exotickik.toml`, then builds its entry file using the manifest's `target` and
`optimasi`. Command-line options override the manifest, and giving an explicit
file path bypasses the manifest entirely:

```
$ exotickik run                 # builds [build].entri from ./exotickik.toml
$ exotickik test                # runs uji_* in the project entry
$ exotickik run -O3             # manifest settings, but -O3 overrides optimasi
$ exotickik run src/other.exo  # explicit file; manifest ignored
```

The manifest parser accepts `[section]` headers, `kunci = "nilai"` string values,
bare integers, and `#` comments. Only `[paket]` (`nama`, `versi`) and `[build]`
(`entri`, `target`, `optimasi`) are read today; unknown keys are ignored so the
schema can grow. Dependency resolution, a package cache, and a registry
(`[dependensi]`, build profiles) remain future work and need network/registry
infrastructure that the bootstrap does not ship.
