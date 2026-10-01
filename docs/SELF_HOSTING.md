# Self-Hosting: Status and Gap Analysis

Self-hosting means the Lime compiler is itself written in Lime and compiled by a
previous Lime compiler. The bootstrap compiler (in Rust) is the starting point;
this document records concrete, evidence-based progress toward replacing it, and
the specific language gaps that still block it.

## What has been demonstrated

Two non-trivial, compiler-shaped programs are now written **entirely in Lime**,
each a `modul` with `publik fungsi evaluasi(teks) -> angka` and its own `lime test`
suite, both pinned by `cli/tests/selfhost.rs` (they evaluate `12 + 34 * 5 = 182`,
`(12 + 34) * 5 = 230`, `1000 / 5 / 2 = 100`).

- `examples/kalkulator/` — a byte-scanning **lexer** producing a `daftar<Token>`
  (each `Token` carrying an `enum Jenis` kind) and a **precedence-climbing
  evaluator** that dispatches on the token kind with `cocok`. Evaluates inline,
  without building a tree.
- `examples/pohon/` — a two-phase mini-compiler: the same lexer, a
  **recursive-descent parser that builds a real AST**, and a separate evaluator
  that walks it. The AST is an **arena**: nodes live in a `daftar<Node>` and refer
  to their children by integer index (`kiri`/`kanan`), so a genuine tree is built
  and traversed with no heap pointers. The parser threads the arena as a
  `&ubah daftar<Node>` through recursion; the evaluator recurses over
  `&daftar<Node>`, dispatching on each node's `enum Jenis` with `cocok`.

`examples/pohon/` is the important result: **Lime can already build and traverse a
real AST today**, which is the heart of a compiler front end.

### What this proves Lime can already express

The evaluator exercises, in Lime, the machinery a compiler front end is built
from:

- **Lexing**: `teks` byte access (`bita_pada`), classification, and numeric
  accumulation over a `selama` loop.
- **Token storage**: a `tipe Token { … }` struct (with an `enum Jenis` kind
  field) held in a growable `daftar<Token>`, built with `dorong` and read with
  `elemen`.
- **Dispatch**: `cocok` on each token's `Jenis` for precedence and operator
  application, with exhaustiveness checked at compile time.
- **Recursion with references**: parse functions take `&daftar<Token>` and
  recurse (precedence climbing), threading position/result through a Copy
  `Langkah` struct — no global mutable state needed.
- **Modules + tooling**: the module is consumed by both a demo and a test file,
  and the whole thing builds, runs, formats (`lime fmt`), and tests
  (`lime test`) through the normal toolchain.

So Lime can express scanning and evaluation. The compiler's **tree-building
core** is what still needs new language features.

## Gaps blocking a self-hosted compiler (evidence-based)

A real compiler needs an AST and an IR — recursive, tagged tree types — plus
reusable containers and string handling. The following were verified against the
current compiler.

1. **Sum types: enum variants with associated data.** `cocok` pattern matching
   over enum variants now exists (statement form, with exhaustiveness checking
   and an optional `lain` default — see `docs/LANGUAGE_SPEC.md`), so dispatch on a
   node's kind is already expressible:
   ```lime
   cocok simpul {
       Angka => { ... }
       Biner => { ... }
   }
   ```
   What remains is **associated data**: an AST node is a tagged union that carries
   payloads (`Number(i64)`, `Binary(op, lhs, rhs)`), and today `enum` variants are
   payload-less:
   ```
   enum Pohon { Daun(angka) Cabang }
   // error[E0110]: diharapkan pengenal, ditemukan '('
   ```
   Payload-carrying variants plus binding patterns in `cocok` would let an AST node
   carry its data *inline* and be matched idiomatically. This is a convenience, not
   a hard blocker: `examples/pohon/` already builds a real AST using a fixed `Node`
   struct plus an arena (see gap 2).

2. **Heap-recursive data types — a convenience, not a blocker.** A recursive
   struct *type-checks* through an owned box — `tipe Simpul { data: angka, berikut:
   kotak<Simpul> }` is accepted — but cannot be *constructed*, because `kotak<T>`
   is non-nullable and there is no optional/variant to terminate the recursion.
   **However, a tree does not require heap-recursive types:** `examples/pohon/`
   builds a working AST as an **arena** (`daftar<Node>`, children referenced by
   integer index), which Lime expresses today. Nullable boxes / optional variants
   would make pointer-based trees possible too, but the index-arena approach is a
   complete, idiomatic way to write the compiler's data structures now.

3. **Generic structs and enums.** Reusable containers need type parameters on
   data types, not just functions:
   ```
   tipe Pasangan<T> { a: T b: T }
   // error[E0110]: diharapkan '{', ditemukan '<'
   ```
   Generic *functions* already work and monomorphize; generic *types* do not yet.

4. **Maps/sets (`peta<K, V>`, `himpunan<T>`).** Symbol tables, string interning,
   and type environments need associative containers. Only `daftar<T>` exists.

5. **First-class owned strings and richer results.** The compiler builds many
   strings (identifiers, messages). Today text is byte buffers (`daftar<bita>`)
   plus the `Teks` module; there is no owned `teks` value type, slicing, or
   string-keyed map. `Hasil<T, R>` also requires Copy payloads, so error values
   cannot yet carry owned messages.

## Suggested order to close the gaps

The lexer, a real arena-based AST, and a recursive evaluator are **already
demonstrated** (`examples/pohon/`). The remaining work is mostly about ergonomics
and scale, in roughly this order:

1. `peta<K, V>` map — the most load-bearing missing container (symbol tables,
   string interning, type environments). Arena ASTs work without it, but name
   resolution needs it.
2. Owned `teks` strings + `Hasil` with owned payloads — identifiers and
   diagnostic messages.
3. Generic structs/enums — reusable typed containers instead of per-type structs.
4. Enum associated data + binding patterns in `cocok` — inline node payloads, an
   ergonomic alternative to the arena `Node` struct (tag matching already works).
5. Port the bootstrap front end to Lime, component by component (lexer and an AST
   parser/evaluator are demonstrated; next: a full parser → AST, then sema, then
   IR), each validated with `lime test` against the Rust bootstrap's behavior.

These are tracked in `docs/ROADMAP.md`. `examples/pohon/` stands as executable
proof that a lexer, a real AST, and a tree-walking evaluator are all expressible
in Lime today.
