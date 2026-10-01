# Lime Toolchain

## The `lime` command

```
lime build <file>     kompilasi ke executable native
lime run <file>       kompilasi lalu jalankan
lime check <file>     periksa sintaks dan tipe tanpa kompilasi
lime test <file>      jalankan fungsi uji ('uji_*') di file
lime fmt <file>       format kode sumber (tambahkan --write untuk menimpa)
lime emit-ir <file>   cetak LLVM IR yang dihasilkan
lime init [nama]        buat proyek Lime baru (lime.toml + src/main.lime)
lime clean              hapus artefak build (.lime-build)
lime version            tampilkan versi
lime help               bantuan
```

Common options for `build` / `run` / `check`:

| Option | Meaning |
|---|---|
| `-o, --output <file>` | output executable name |
| `--target <name>` | `host` (default), `linux-x64`, `windows-x64` |
| `-O0 -O1 -O2 -O3` | optimization level passed to the backend (default `-O2`) |
| `-- <args...>` | arguments forwarded to the program under `lime run` |

## Testing (`lime test`)

`lime test <file>` discovers and runs the test functions in a file. A test
function is any top-level function whose name begins with `uji_`, takes no
parameters, and returns `boolean` — `benar` means the test passed:

```lime
program UjiMatematika
gunakan Matematika

fungsi uji_pangkat_dua() -> boolean {
    kembalikan pangkat(2, 10) == 1024
}
```

```
$ lime test examples/uji_matematika.lime
menjalankan 5 uji: examples/uji_matematika.lime

  ok     uji_pangkat_dua
  ok     uji_fpb
  ...

hasil: 5 lulus, 0 gagal
```

The test file must not contain a `utama` block — the runner synthesizes one that
calls each `uji_*` function and reports its result. Tests are compiled and run as
one native executable (so they exercise the real backend), with output flushed
after each test so a crash in one test still reports the ones that ran before it;
such a test is listed as `berhenti sebelum selesai`. `lime test` exits non-zero
if any test fails or aborts. A `uji_*` function with parameters, type parameters,
or a non-`boolean` return type is a compile error.

## Benchmarking (`lime bench`)

`lime bench <file>` measures benchmark functions. A benchmark is any top-level
function whose name begins with `tolok_`, takes no parameters, and returns
`angka` — the return value is summed into a checksum so the optimizer cannot
delete the work being measured:

```lime
program TolokMatematika
gunakan Matematika

fungsi tolok_pangkat() -> angka {
    kembalikan pangkat(3, 15)
}
```

```
$ lime bench examples/tolok_matematika.lime --iterasi 100000
menjalankan 3 tolok ukur: examples/tolok_matematika.lime (100000 iterasi)

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
`berhenti sebelum selesai` and `lime bench` exits non-zero.

## Build pipeline

`lime build` runs: lex → parse → semantic analysis → Lime IR lowering →
Lime IR optimization → LLVM IR emission → `llc` (object) → linker (executable).

Intermediate files (`<program>.ll`, `<program>.o`) are written under a
`.lime-build/` directory next to the output. The backend locates tools in this
order (environment override first):

| Purpose | Env override | Fallback candidates |
|---|---|---|
| IR → object | `LIME_LLC` | `llc`, `llc-18`, `llc-17` |
| link (native ABI) | `LIME_CC` | `cc`, `clang`, `gcc` |
| link (Windows) | `LIME_LINKER` | `lld-link` |

No internet access is required for compilation.

## Projects and the manifest (`lime.toml`)

`lime init [nama]` scaffolds a project: a `lime.toml` manifest and
`src/main.lime`.

```toml
[paket]
nama = "halo"
versi = "0.1.0"

[build]
entri = "src/main.lime"   # entry file (default: src/main.lime)
target = "host"           # host | linux-x64 | windows-x64
optimasi = 2              # 0..3 (default 2)
```

When `build`, `run`, `check`, `test`, or `bench` is invoked **without a file
argument**, Lime searches the current directory and its ancestors for a
`lime.toml`, then builds its entry file using the manifest's `target` and
`optimasi`. Command-line options override the manifest, and giving an explicit
file path bypasses the manifest entirely:

```
$ lime run                 # builds [build].entri from ./lime.toml
$ lime test                # runs uji_* in the project entry
$ lime run -O3             # manifest settings, but -O3 overrides optimasi
$ lime run src/other.lime  # explicit file; manifest ignored
```

The manifest parser accepts `[section]` headers, `kunci = "nilai"` string values,
bare integers, and `#` comments. Only `[paket]` (`nama`, `versi`) and `[build]`
(`entri`, `target`, `optimasi`) are read today; unknown keys are ignored so the
schema can grow. Dependency resolution, a package cache, and a registry
(`[dependensi]`, build profiles) remain future work and need network/registry
infrastructure that the bootstrap does not ship.
