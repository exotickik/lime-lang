# Phase 26 (Arena AST in Lime) — Design Assessment

Baseline: `main` after `cocok` pattern matching (Phase 25). The self-hosting gap
analysis listed recursive/heap data types as a blocker for an AST. This phase
tests that assumption with evidence and, finding it wrong, delivers a real
AST-building compiler in Lime — with no new language features.

## The insight

A compiler's AST is usually a heap-recursive, owned, tagged tree, and Lime lacks
owned enum payloads and nullable boxes. But a tree does **not** require heap
recursion: a data-oriented **arena** stores every node in one growable array and
has nodes refer to their children by integer index. That needs only structs, an
enum tag, a `daftar`, and recursion — all shipped. So the "recursive types block
the AST" gap was overstated.

## What was built

`examples/pohon/` — a two-phase arithmetic compiler, as a `modul` with
`publik fungsi evaluasi(teks) -> angka`, a demo, and a `lime test` suite:

1. **Lexer** → `daftar<Token>`, each `Token` carrying an `enum Jenis` kind.
2. **Parser** (precedence climbing) that **builds an AST** rather than evaluating:
   nodes are `tipe Node { jenis: Jenis, data: angka, kiri: angka, kanan: angka }`
   pushed into an arena `daftar<Node>`; `kiri`/`kanan` are child indices (`-1` for
   a leaf). `tambah_node` appends and returns the new index. The arena is threaded
   through the recursive parser as a `&ubah daftar<Node>`, alongside the read-only
   `&daftar<Token>` and a position carried in a Copy `Langkah { pos, idx }` struct.
3. **Evaluator** `hitung(&daftar<Node>, idx)` — recurses over the arena,
   dispatching on each node's `Jenis` with `cocok`.

`evaluasi` lexes, parses into the arena, then walks the tree. Results match the
inline calculator (182 / 230 / 98 / 56 / 100), proving the AST is built and
traversed correctly.

## Why this matters for self-hosting

This is the heart of a compiler front end — turning tokens into a tree and walking
it — written entirely in Lime, today. It shows the language is already expressive
enough for a parser-to-AST and a tree pass; what remains for a full self-hosted
compiler is scale and ergonomics (a `peta` map for symbol tables, owned strings
for identifiers/diagnostics), not a missing ability to represent trees.

Notably, the parser simultaneously holds a shared borrow of the token list and a
mutable borrow of the arena (two different `daftar`s) and threads both through
recursion — exercising the borrow checker on realistic compiler code without
tripping it.

## Tests

- `cli/tests/selfhost.rs`: the AST calculator evaluates the five expressions
  correctly and passes its own six-test `lime test` suite (alongside the existing
  inline calculator's tests).
- `examples/pohon/uji.lime`: the in-language suite.

All prior tests still pass; 0 warnings; the repo still passes `fmt --check`.

## Honest limitations

- Still arithmetic only; the point is the architecture (lexer → AST → walk), not
  the language it compiles.
- The arena `Node` is a fixed struct holding the union of all node shapes
  (`data`, `kiri`, `kanan`); inline per-variant payloads await enum associated
  data (Phase 25 follow-up). This is an ergonomic nicety, not a blocker.
- Name resolution / symbol tables still need a `peta<K, V>` map (next priority);
  identifiers and messages still need owned strings. See `docs/SELF_HOSTING.md`.
