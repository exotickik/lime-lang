# GitHub Linguist eligibility — Exotickik

Linguist adds a new language only once it has **sufficient real-world usage** on
public GitHub. This file tracks Exotickik's status against the upstream rule so
the contribution can be submitted the moment it qualifies — and not before.

## Upstream requirement (verified against `github-linguist/linguist`)

From `CONTRIBUTING.md`, section *"Language extension and filename usage
requirements"* (checked at linguist commit `5fbdfcb`, 2026-10-03):

> - at least **2000 files per extension** indexed in the last year (the number at
>   the top of the search results), **excluding forks**, for extensions expected
>   to occur more than once per repo.
> - the results should show a **reasonable distribution across unique
>   `:user/:repo` combinations**. If particular users dominate (e.g. the primary
>   language owner), those users are filtered out with `-user:<name>`.

Assessment uses GitHub code search. The PR must link to a search showing in-the-wild usage.

`.exo` is currently **unassigned** in `languages.yml` (no conflicting language),
so only the "adding a language" path applies — no disambiguation heuristic needed.

## Current status: NOT YET ELIGIBLE

Exotickik is a new language. The `.exo` files in public GitHub are nowhere near
2000 across a diverse set of repositories, and most existing `.exo` hits on GitHub
belong to unrelated tools, not Exotickik source. Opening a PR now would be closed
per the policy ("we do not accept PRs for very new or hobby languages").

**Do not open the upstream PR until the search evidence below clears the bar.**

> Note: the preparation environment could not query GitHub code search
> programmatically (the session's API is repo-scoped and the global
> `search/code` endpoint returned HTTP 403). The counts below must be read
> manually from the browser search URLs at submission time.

## Evidence to collect at submission time

Run these and record the counts + a manual distribution check:

- Primary search (paste the result count and URL):
  `https://github.com/search?q=path%3A*.exo+%28fungsi+OR+kembalikan+OR+tampilkan%29+-is%3Afork&type=code`
- Broaden/adjust the keyword set to Exotickik-specific tokens (`fungsi`, `kembalikan`,
  `tampilkan`, `cocok`, `milik`/`salinan`, `daftar<`, `peta<`) to exclude unrelated
  `.exo` files from other ecosystems.
- Confirm the hits span many distinct `:user/:repo` (not dominated by
  `exotickik/*`); if needed, re-run with `-user:exotickik`.

## Checklist before opening the PR

- [ ] ≥ 2000 non-fork `.exo` files indexed in the last year
- [ ] Reasonable distribution across unique `:user/:repo`
- [ ] Hits are genuinely Exotickik source (not unrelated `.exo` formats)
- [ ] Grammar added via `script/add-grammar` and committed
- [ ] `languages.yml` entry added in alphabetical position
- [ ] `script/update-ids` run to assign `language_id`
- [ ] ≥ 1 representative, non-trivial sample in `samples/Exotickik/`
- [ ] Sample licensing stated (MIT — see `PR.md`)
- [ ] Linguist test suite passes locally
- [ ] Upstream PR template fully filled in

Until every box is checked, Exotickik is prepared but **not** submitted.
