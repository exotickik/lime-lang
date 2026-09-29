# Lime Toolchain

## The `lime` command

```
lime build <berkas>     kompilasi ke executable native
lime run <berkas>       kompilasi lalu jalankan
lime check <berkas>     periksa sintaks dan tipe tanpa kompilasi
lime fmt <berkas>       format kode sumber (tambahkan --write untuk menimpa)
lime emit-ir <berkas>   cetak LLVM IR yang dihasilkan
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
