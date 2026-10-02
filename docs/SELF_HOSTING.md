# Self-Hosting: Status and Gap Analysis

Self-hosting means the Exotickik compiler is itself written in Exotickik and compiled by a
previous Exotickik compiler. The bootstrap compiler (in Rust) is the starting point;
this document records concrete, evidence-based progress toward replacing it, and
the specific language gaps that still block it.

## What has been demonstrated

Two non-trivial, compiler-shaped programs are now written **entirely in Exotickik**,
each a `modul` with `publik fungsi evaluasi(teks) -> angka` and its own `exotickik test`
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

`examples/pohon/` is the important result: **Exotickik can already build and traverse a
real AST today**, which is the heart of a compiler front end.

### What this proves Exotickik can already express

The evaluator exercises, in Exotickik, the machinery a compiler front end is built
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
  and the whole thing builds, runs, formats (`exotickik fmt`), and tests
  (`exotickik test`) through the normal toolchain.

So Exotickik can express scanning and evaluation. The compiler's **tree-building
core** is what still needs new language features.

## Gaps blocking a self-hosted compiler (evidence-based)

A real compiler needs an AST and an IR — recursive, tagged tree types — plus
reusable containers and string handling. The following were verified against the
current compiler.

1. **Sum types: enum variants with associated data.** ✅ Done (Phase 31). `cocok`
   pattern matching over enum variants (statement form, exhaustiveness checking,
   optional `lain` default) now combines with **associated data**: a variant
   carries positional payloads and a `cocok` arm binds them:
   ```exotickik
   enum Token { Angka(angka) Operator(char) Akhir }

   cocok t {
       Angka(n) => { ... }        // n bound from the payload
       Operator(c) => { ... }
       Akhir => { ... }
   }
   ```
   An AST node can now carry its data *inline* (`Angka(angka)`,
   `Biner(Operator, kiri, kanan)`) instead of through the arena `Node` struct of
   gap 2 — though the arena approach remains valid. Payloads are currently Copy
   (integers, `char`, `boolean`, `teks`, enums, Copy structs — which covers token
   kinds, literals and arena indices) **and Owned** (`daftar`, `kotak`, `peta`,
   owned structs/enums): an enum with an owned payload is itself Owned, moves,
   and drops by freeing only the active variant's payload (tag-conditional,
   valgrind-clean). **Generic enums** also work (`Pilihan<T> { Ada(T) Kosong }`,
   monomorphized), including owned type arguments (`Pilihan<daftar<angka>>`). The
   remaining container gap is owned *element types* in `daftar<T>`.

2. **Heap-recursive data types — a convenience, not a blocker.** A recursive
   struct *type-checks* through an owned box — `tipe Simpul { data: angka, berikut:
   kotak<Simpul> }` is accepted — but cannot be *constructed*, because `kotak<T>`
   is non-nullable and there is no optional/variant to terminate the recursion.
   **However, a tree does not require heap-recursive types:** `examples/pohon/`
   builds a working AST as an **arena** (`daftar<Node>`, children referenced by
   integer index), which Exotickik expresses today. Nullable boxes / optional variants
   would make pointer-based trees possible too, but the index-arena approach is a
   complete, idiomatic way to write the compiler's data structures now.

3. **Generic data types.** ✅ Generic **structs** (`tipe Pasangan<T, U> { … }`,
   see `examples/pasangan.exo`), payload-carrying enum variants (Copy and owned,
   see `examples/nilai_dinamis.exo`), and generic **enums** (`enum Pilihan<T> {
   Ada(T) Kosong }`, see `examples/pilihan.exo`) all work now, monomorphized with
   no runtime cost. The remaining container gap is owned *element types* inside
   `daftar<T>`.

4. **Maps/sets.** ✅ `peta<K, V>` (a real string-keyed hash table) now exists —
   symbol tables, string interning, and module tables are expressible (see
   `examples/peta.exo`). A `himpunan<T>` set and integer-keyed maps remain
   follow-ups.

5. **First-class owned strings and richer results.** Owned, growable, mutable
   strings exist as `daftar<bita>` byte buffers, and the `Teks` module now covers
   building, comparison, search, prefix-match and slicing (`sama_teks`,
   `sama_bita`, `cari_bita`, `awalan`, `potong`) — the lexer string operations.
   `Hasil<T, R>` now supports **owned payloads** (so a function can return owned
   bytes/collections on success or an owned error, and `coba`-propagate them). What
   remains is a first-class owned `teks` *value* type with a buffer→`teks`
   conversion, and owned-payload inspection via pattern matching (the inspection
   builtins stay Copy-only; owned results are consumed with `coba`).

## Suggested order to close the gaps

The lexer, a real arena-based AST, and a recursive evaluator are **already
demonstrated** (`examples/pohon/`). The remaining work is mostly about ergonomics
and scale, in roughly this order:

1. ✅ `peta<K, V>` map — done (Phase 27). String-keyed symbol tables and interning
   are now expressible.
2. Owned `teks` strings + `Hasil` with owned payloads — identifiers and
   diagnostic messages (next priority).
3. Generic structs/enums — reusable typed containers instead of per-type structs.
4. ✅ Enum associated data + binding patterns in `cocok` (Phase 31) — inline node
   payloads, an ergonomic alternative to the arena `Node` struct. Owned payloads
   and generic enums remain.
5. Port the bootstrap front end to Exotickik, component by component (lexer and an AST
   parser/evaluator are demonstrated; next: a full parser → AST, then sema, then
   IR), each validated with `exotickik test` against the Rust bootstrap's behavior.

These are tracked in `docs/ROADMAP.md`. `examples/pohon/` stands as executable
proof that a lexer, a real AST, and a tree-walking evaluator are all expressible
in Exotickik today.
