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

## Package manifest (`lime.toml`)

`lime init` scaffolds a manifest. The format is defined now so the package
manager (roadmap Phase 7) can adopt it without breaking projects:

```toml
[paket]
nama = "halo"
versi = "0.1.0"

[build]
target = "host"
```

Planned sections: `[dependensi]` (dependencies), `[profil.rilis]` (build
profiles). Dependency resolution, a package cache and a registry are future
work; the manifest schema is intentionally stable from the start.
