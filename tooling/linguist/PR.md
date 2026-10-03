# Upstream PR (prepared — DO NOT OPEN until eligibility is met)

Target: `github-linguist/linguist`, default branch, via a fork (Linguist uses the
standard fork + PR workflow; the Exotickik repo's no-branch rule does not apply to
their repository).

## PR title

    Add Exotickik language

## PR body (fill the upstream template)

**Description**

Adds the Exotickik programming language — an independent, natively compiled
language with Indonesian keywords, an ownership/borrow model, generics, sum types,
and an LLVM backend. Source files use the `.exo` extension.

- `languages.yml`: new `Exotickik` entry (`type: programming`, `color: #8acb1c`,
  `extensions: [".exo"]`, `tm_scope: source.exotickik`, `ace_mode: text`,
  `language_id` generated via `script/update-ids`).
- Grammar: `source.exotickik` added with `script/add-grammar` from the Exotickik
  grammar (MIT-licensed; see below).
- Samples: real-world Exotickik source under `samples/Exotickik/` (see licensing).

**Checklist** (from CONTRIBUTING.md)

- [ ] I am adding a new language.
- [ ] The extension `.exo` has sufficient usage: <paste search count + URL>.
- [ ] I have added a grammar with `script/add-grammar`.
- [ ] The grammar license is one of the accepted licenses (MIT).
- [ ] I have added at least one non-trivial, real-world sample.
- [ ] I ran `script/update-ids`.
- [ ] I ran `bundle exec rake test` (or `rspec`) and it passes.

**Usage evidence**

<Paste the GitHub code-search result count (must be ≥ 2000 non-fork `.exo`
files indexed in the last year) and the search URL. Confirm a reasonable
distribution across unique `:user/:repo`. See ELIGIBILITY.md.>

Example search:
`https://github.com/search?q=path%3A*.exo+%28fungsi+OR+kembalikan+OR+tampilkan%29+-is%3Afork&type=code`

**Grammar source**

The TextMate grammar lives in the Exotickik repository at
`tooling/grammar/exotickik.tmLanguage.json` (scope `source.exotickik`), MIT
licensed. For `script/add-grammar`, point at the grammar's repository URL.

**Sample licensing**

The samples under `samples/Exotickik/` are taken from the Exotickik repository's
own compiler and examples and are MIT licensed (same as Linguist). Sources:
`compiler/*.exo` and `examples/*.exo` in the Exotickik repository.

## Steps to execute at submission time

```bash
# in a fork of github-linguist/linguist
script/add-grammar https://github.com/<exotickik-grammar-repo>
# add the Exotickik entry to lib/linguist/languages.yml (alphabetical)
script/update-ids
mkdir -p samples/Exotickik
cp <exotickik>/compiler/Lekser.exo samples/Exotickik/Lekser.exo
cp <exotickik>/examples/nilai_dinamis.exo samples/Exotickik/nilai_dinamis.exo
bundle exec rake test
# open the PR using the upstream template, pasting the search evidence
```
