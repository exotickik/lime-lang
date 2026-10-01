# Phase 23 (Formatter tooling `lime fmt --check`) — Design Assessment

Baseline: `main` after the project manifest (Phase 22). Eighth prerequisite item:
developer tooling, implemented to the extent that can be honestly validated in a
headless Linux environment.

## Goal

Make the formatter usable in real workflows and CI: check formatting without
writing, format many files or a whole project at once, and integrate with the
`lime.toml` project model from Phase 22.

## What is honestly possible here

- **Possible now:** a CI-grade `fmt --check`, multi-file and recursive directory
  formatting, and project-wide formatting via `lime.toml`. These run and are
  verified end-to-end headlessly.
- **Not validatable here:** a Language Server (LSP) and editor extensions. They
  require an editor/client to exercise, which this environment has no way to
  drive, so building one now could not be honestly tested. It stays an open
  roadmap item rather than an untested stub. The formatter already exposes the
  reusable core (`lime_fmt::format_source`) that such a server would call.

## Behavior

`lime fmt` resolves its targets then operates in one of three modes:

- **Targets.** Explicit files are used as-is; a directory is walked recursively
  for `*.lime` (skipping `target`, `.lime-build`, `.git`); with no path, the
  project is formatted — the directory of the nearest `lime.toml` (reusing the
  Phase 22 `manifest::find` ancestor search).
- **`--check`** — read and format each file in memory, print `perlu format: <f>`
  for any whose formatted form differs, and exit non-zero if any do. Nothing is
  written. This is the CI gate.
- **`--write`** — rewrite in place, but only files that actually change (so clean
  files are not touched and mtimes are stable); each rewrite is reported.
- **Neither flag** — print the formatted result to stdout; allowed only for a
  single file (multiple targets without a mode is an error, since stdout cannot
  represent them unambiguously).

`--write` and `--check` are mutually exclusive.

## Implementation

- `cli/src/main.rs`: rewritten `cmd_fmt` (flag parsing, target resolution,
  three modes), `fmt_one` (read + format one file, reporting parse errors), and
  `collect_lime_files` (recursive `*.lime` walk with ignored directories).
- Reuses `manifest::find` for project mode and `lime_fmt::format_source` for the
  actual formatting — no new formatting logic, so idempotence and
  semantics-preservation are unchanged.

## Dogfooding

Running `lime fmt --check` across the repository surfaced six example files that
predated the current formatter conventions; they were reformatted with
`lime fmt --write` and re-run to confirm unchanged behavior. The repository now
passes its own `fmt --check`.

## Tests

- `cli/tests/fmt_cmd.rs`: `--check` flags an unformatted file (and does not
  modify it), `--check` passes a formatted file, `--write` is idempotent,
  recursive directory formatting finds nested files, single-file stdout mode does
  not modify the file, multiple files without a mode is an error, and project-mode
  formats the manifest directory.

## Honest limitations

- No LSP / editor integration (cannot be validated headlessly; see above).
- No diff output for `--check` (it lists files, not the specific changes).
- No `.limeignore`-style configuration beyond the fixed skip list.
- No parallelism across files (fine for current project sizes).
