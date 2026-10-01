# Phase 22 (Project manifest `lime.toml`) — Design Assessment

Baseline: `main` after FFI expansion (Phase 21). Seventh prerequisite item: a
package/project system, implemented to the extent possible without a network or a
registry.

## Goal

Make `lime.toml` functional: run the toolchain against a project without naming a
file every time, taking the entry point and build settings from the manifest.
This is the part of a package system that is genuinely implementable offline; the
registry/dependency part is not, and is documented as such.

## What is honestly possible here

- **Possible now:** a manifest that names the project and its entry file, chooses
  the target and optimization level, and is discovered from the working directory
  (or an ancestor). The whole toolchain already compiles multi-file projects via
  the module system (`impor`), so a "project" is just an entry file plus its
  imports.
- **Not possible offline:** dependency resolution, a package cache, and a
  registry. These need infrastructure the bootstrap does not ship, so they are
  left explicitly as future work rather than stubbed.

## Manifest format

A small TOML subset, enough for the schema and stable for growth:

```toml
[paket]
nama = "halo"
versi = "0.1.0"

[build]
entri = "src/main.lime"   # default src/main.lime
target = "host"           # host | linux-x64 | windows-x64
optimasi = 2              # 0..3
```

Parsing supports `[section]` headers, `kunci = "nilai"` strings, bare integers,
and `#` comments (respecting `#` inside quoted strings). Only `[paket]` and
`[build]` keys are read; unknown keys are ignored so the schema can extend
without breaking older tools.

## Resolution model — file mode vs project mode

The command-line argument parser was split into raw parsing (`parse_args`, no
defaults, every option an `Option`) and resolution (`parse_common`):

- **File mode** — a file path is given: use it, with CLI flags and the built-in
  defaults (`-O2`, host). The manifest is ignored, so `lime run x.lime` behaves
  identically everywhere.
- **Project mode** — no file path: search the current directory and its ancestors
  for `lime.toml`, load it, and build its entry file. `target` and `optimasi`
  come from the manifest unless the corresponding CLI flag is present (CLI wins).
  A missing manifest or a missing entry file is a clear error.

This precedence — explicit CLI flag > manifest > built-in default, and explicit
file > manifest entry — keeps behavior predictable. All five build-like commands
(`build`, `run`, `check`, `test`, `bench`) share it.

## Implementation

- `cli/src/manifest.rs`: the `Manifest` struct, `find()` (ancestor search),
  `load()` (the TOML-subset parser), and `entry_path()`.
- `cli/src/main.rs`: `parse_args` (raw) + `parse_common` (resolution with
  file/project modes); `target_by_name` centralizes target validation.

## Tests

- `cli/src/manifest.rs` unit tests: field/section parsing, comments and blank
  lines, `#` inside strings, default entry, and a rejected out-of-range
  `optimasi`.
- `cli/tests/project_cmd.rs`: `init` then project-mode `run`, a custom `entri`,
  discovery from a subdirectory, a clear error with no manifest, a clear error
  for a missing entry, and an explicit file overriding project mode.

## Honest limitations

- No dependencies, lockfile, package cache, or registry (offline bootstrap).
- No build profiles (`[profil.*]`) or per-profile optimization yet.
- Strings only for scalar values; no arrays/tables beyond the two known sections.
- `fmt` still takes an explicit file (it is not a build command).
