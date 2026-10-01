# Self-Hosting: Status and Gap Analysis

Self-hosting means the Lime compiler is itself written in Lime and compiled by a
previous Lime compiler. The bootstrap compiler (in Rust) is the starting point;
this document records concrete, evidence-based progress toward replacing it, and
the specific language gaps that still block it.

## What has been demonstrated

A non-trivial, compiler-shaped program is now written **entirely in Lime** and
passes its own tests: an arithmetic expression evaluator.

- `examples/kalkulator/Kalkulator.lime` — a `modul` with `publik fungsi
  evaluasi(teks) -> angka`: a byte-scanning **lexer** that produces a
  `daftar<Token>`, and a **precedence-climbing parser/evaluator** that honors
  operator precedence, parentheses, and left-associativity.
- `examples/kalkulator/main.lime` — a demo entry (`impor Kalkulator`).
- `examples/kalkulator/uji.lime` — six `uji_*` tests run by `lime test`.

It is pinned by `cli/tests/selfhost.rs` and evaluates e.g. `12 + 34 * 5 = 182`,
`(12 + 34) * 5 = 230`, `1000 / 5 / 2 = 100`.

### What this proves Lime can already express

The evaluator exercises, in Lime, the machinery a compiler front end is built
from:

- **Lexing**: `teks` byte access (`bita_pada`), classification, and numeric
  accumulation over a `selama` loop.
- **Token storage**: a `tipe Token { … }` struct held in a growable
  `daftar<Token>`, built with `dorong` and read with `elemen`.
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

1. **Sum types: enum variants with associated data + pattern matching (`cocok`).**
   An AST node is a tagged union (a `Number(i64)`, a `Binary(op, lhs, rhs)`, …).
   Today `enum` is C-style only:
   ```
   enum Pohon { Daun(angka) Cabang }
   // error[E0110]: diharapkan pengenal, ditemukan '('
   ```
   This is the single largest blocker: without payload-carrying variants and
   `cocok` destructuring, an AST/IR cannot be represented idiomatically. The
   calculator sidesteps it by evaluating inline and never building a tree.

2. **Constructible recursive types.** A recursive struct *type-checks* through an
   owned box — `tipe Simpul { nilai_x: angka, berikut: kotak<Simpul> }` is
   accepted — but it cannot be *constructed*, because `kotak<T>` is non-nullable
   and there is no optional/variant to terminate the recursion. Finite trees and
   lists therefore need either sum types (gap 1) or a nullable/optional box.

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

1. Enum associated data + `cocok` pattern matching (unblocks AST/IR).
2. Generic structs/enums (unblocks reusable containers and a typed AST).
3. `peta<K, V>` map (symbol tables).
4. Owned `teks` strings + `Hasil` with owned payloads (identifiers, diagnostics).
5. Then port the bootstrap front end to Lime, component by component (lexer is
   demonstrated; parser → AST, then sema, then IR), each validated with
   `lime test` against the Rust bootstrap's behavior.

These are tracked in `docs/ROADMAP.md`. The calculator stands as the executable
proof that the non-AST parts of a compiler are already expressible in Lime today.
