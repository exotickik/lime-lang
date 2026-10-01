# Phase 25 (`cocok` pattern matching) — Design Assessment

Baseline: `main` after the first self-hosting component + gap analysis (Phase 24).
The gap analysis named enum associated data + pattern matching as the main
blocker for a self-hosted AST/IR. This phase delivers the pattern-matching half
— `cocok` over enum variants — as a complete, tested feature, and leaves
associated data (payload-carrying variants) as the clearly-scoped next step.

## Why this scope

Full sum types change the enum runtime representation from an `i32` tag to a
tagged union, rippling through every enum code path (equality, printing, fields,
arrays) with real memory-layout risk. Pattern matching on the existing `i32`-tag
enums carries none of that risk: it is tag dispatch, reusing the existing
comparison and branch machinery. It is a genuine, self-contained language feature
— dispatch on a variant is the shape compiler code takes — and it is exactly what
the payload-carrying version later extends, so nothing here is throwaway.

## Semantics

```lime
cocok nilai_enum {
    Varian1 => { ... }
    Varian2 => { ... }
    lain => { ... }        // optional default
}
```

- The scrutinee must be an enum (`E0211` otherwise).
- Each arm names a variant of that enum (`E0265` for an unknown variant, and for
  a variant handled twice).
- Matching must be **exhaustive**: every variant has an arm, or a `lain` default
  is present; otherwise `E0268` names the missing variants.
- Arms are **statements** — they run for effect or `kembalikan`. A value-producing
  `cocok` expression and binding patterns for payloads are future work (tied to
  enum associated data).

## Implementation (all layers)

- **Lexer**: a new `=>` token (`FatArrow`); `cocok` was already a reserved
  keyword.
- **AST**: `Stmt::Match(MatchStmt { scrutinee, arms: Vec<MatchArm>, default })`.
- **Parser**: `parse_cocok` — scrutinee parsed with struct-literals suppressed
  (so the `{` opens the arms), variant/`=>`/block arms, one optional `lain`.
- **Sema** (`check_match`): types the scrutinee, resolves the enum, checks each
  arm variant (unknown/duplicate), and enforces exhaustiveness against the
  variant set unless a `lain` is present. Node renumbering, generic let-type
  substitution, and span lookup were extended for the new statement.
- **Ownership** (`check_match`): mirrors the `if` branch-consistent-move rule —
  a value moved in one arm must be moved in all (a missing `lain` counts as a
  no-move arm), reported as `E0281`.
- **IR** (`lower_match`): lowers to a chain of `tag == const` comparisons and
  conditional branches to per-arm blocks, all joining a single continuation
  block; the `lain` block (or fall-through) handles the no-match path. Reuses
  `Cmp`/`CondBr`/`Br`; no codegen changes were needed (enums are `i32`).
- **Formatter**: prints `cocok`/arms with canonical indentation; idempotent.

## Tests

- `lime_lexer`: the `=>` token.
- `lime_sema`: exhaustive OK, non-exhaustive (`E0268`), `lain` makes exhaustive,
  unknown variant (`E0265`), non-enum scrutinee (`E0211`).
- `cli/tests/enums.rs`: `cocok` dispatching to the right arm, and a `lain`
  default with in-arm mutation.
- `examples/cocok.lime`: a traffic-light state machine driven by `cocok`.

All 304 prior tests still pass; 0 warnings; the repository still passes its own
`fmt --check`.

## Honest limitations

- Statement form only — no value-producing `cocok` expression yet.
- No payload binding patterns, because enum variants carry no data yet. Adding
  associated data (a tagged-union representation) is the next self-hosting step
  and is tracked in `docs/ROADMAP.md` (Phase 9) and `docs/SELF_HOSTING.md`.
- Matches on enums only (the only sum type today); no literal/range/struct
  patterns.
