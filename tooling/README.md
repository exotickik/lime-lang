# Exotickik tooling

Editor and ecosystem assets for the Exotickik language.

- `grammar/exotickik.tmLanguage.json` — the official TextMate grammar
  (scope `source.exotickik`), MIT licensed. Validated with `vscode-textmate`
  against real compiler sources. Shared by GitHub Linguist and editors.
- `vscode/` — a minimal VS Code extension (language id `exotickik`, `.exo`
  files, comments/brackets/auto-closing) that reuses the grammar.
- `linguist/` — the GitHub Linguist registration package: proposed
  `languages.yml` entry, PR body, and the eligibility checklist. See
  `linguist/README.md`. The upstream PR is prepared but held until Exotickik
  meets Linguist's real-world usage threshold.

Official identity: name **Exotickik**, extension **`.exo`**, scope
**`source.exotickik`**, color **`#8acb1c`**.
