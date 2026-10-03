# Exotickik — GitHub Linguist registration package

Everything needed to register Exotickik with
[github-linguist/linguist](https://github.com/github-linguist/linguist) so that
`.exo` files are detected as **Exotickik** and highlighted on GitHub — prepared
and kept current, to be submitted upstream once the usage threshold is met.

## Status

**Not yet submitted.** Exotickik does not yet meet Linguist's real-world usage
requirement (≥ 2000 non-fork `.exo` files indexed in the last year, spread across
repositories). See `ELIGIBILITY.md`. Submitting early would be rejected by policy,
so the PR is fully prepared but held.

## Contents

- `../grammar/exotickik.tmLanguage.json` — the TextMate grammar (scope
  `source.exotickik`), MIT licensed, covering the real current language. Also used
  by the VS Code extension in `../vscode/`.
- `languages.yml.entry` — the proposed `languages.yml` entry (minus `language_id`,
  which `script/update-ids` assigns).
- `ELIGIBILITY.md` — the upstream usage requirement, how to measure it, and the
  pre-submission checklist.
- `PR.md` — the prepared PR title, body, and the exact commands to run in a
  Linguist fork at submission time.

## Official language identity

- Name: **Exotickik**
- Extension: **`.exo`**
- TextMate scope: **`source.exotickik`**
- Color: **`#8acb1c`** (the official Exotickik brand color — keep consistent)
- `ace_mode`: `text`

## Samples

Linguist requires representative, real-world samples (not hello-world). Use the
Exotickik repository's own files, all MIT licensed:

- `compiler/Lekser.exo`, `compiler/Parser.exo`, `compiler/Token.exo` — the
  self-hosted compiler front end (real, non-trivial usage).
- `examples/nilai_dinamis.exo` — owned sum types over `daftar`/`peta`/`kotak`.
- `examples/pasangan.exo` — generic structs.
- `examples/pilihan.exo` — a generic enum (`Pilihan<T>`).

At submission time copy a couple of these into `samples/Exotickik/` in the
Linguist fork (see `PR.md`).

## Why not `.gitattributes`

GitHub's language bar can be forced with `*.exo linguist-language=...`, but that
is a per-repo lie, not language support. Exotickik is registered properly through
Linguist instead; until then GitHub may show the repo as mostly Rust, which is
expected and correct.
