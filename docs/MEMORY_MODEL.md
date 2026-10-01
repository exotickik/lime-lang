# Lime Memory Model

Lime targets performance-sensitive native software. It has **no garbage
collector** and no hidden background runtime. Memory management is deterministic
and, where possible, resolved at compile time.

## Principles

1. **No garbage collector.** Cleanup is decided entirely at compile time and
   happens at deterministic points; there is no background collector and no
   finalizer thread.
2. **Two value categories, inferred by the compiler.** Every type is either
   **`salinan` (Copy)** or **`milik` (Owned)**. The category is a property of the
   type — there is no `Copy` opt-in trait and no move/copy sigil at use sites.
3. **Stack-first, explicit heap.** Copy values and fixed aggregates live inline
   (stack / registers, promoted by `llc -O2`). The heap is entered only through
   an Owned type; there are no hidden allocations.
4. **Compile-time enforcement, zero runtime ownership cost.** Move validity and
   drop points are proven statically. There are **no runtime drop flags** and no
   reference counts.

## The `salinan` / `milik` split

A type is **Owned (`milik`)** iff it is `kotak<T>` (see below) or an aggregate
(struct or array) that transitively contains an Owned type. Every other type —
all integers, floats, `boolean`, `karakter`, `teks`, `hampa`, enums, and
aggregates built only from these — is **Copy (`salinan`)**.

- **Copy values** behave exactly as before this phase: assignment, argument
  passing, and return **copy** them; they own no resource and are never dropped.
- **Owned values** **move**: assignment, argument passing, returning, and placing
  one into a struct/array literal or a `kotak(...)` transfer ownership and leave
  the source binding **moved-out**. Using a moved-out binding is a compile error
  (`E0280`, *nilai ini sudah dipindahkan*).

## `kotak<T>` — the owned heap cell

`kotak<T>` is the primitive Owned type: a single `T` on the heap, owned by
exactly one binding.

- `kotak(x)` allocates (libc `malloc`), moves `x` in, and yields a `kotak<T>`.
- `isi(k)` reads a **copy** of the contents (requires `T` to be Copy); it borrows
  `k` without consuming it, and also accepts a reference (`&kotak<T>` /
  `&ubah kotak<T>`) so a box can be read through a borrow.
- When a `kotak<T>` goes out of scope while still owning, it is dropped: if `T`
  is itself Owned its contents are dropped first, then the allocation is released
  (libc `free`). Structs/arrays containing Owned fields drop those fields
  recursively.

## Move rules

- **Whole-value moves only.** Moving an Owned field *out of* a struct/array (a
  partial move) is not allowed; read Copy fields freely, move the aggregate whole.
- **Branch-consistent moves.** Within a scope a binding must have the same
  moved/live status on every path that rejoins. Moving in one `jika` arm but not
  another is `E0281` (*nilai dipindahkan pada sebagian cabang saja*). This is why
  drops need no runtime flags — liveness at each program point is statically
  definite.
- **No moves out of a loop.** A loop body may not move a binding declared outside
  the loop (`E0282`); it would be moved on the next iteration. Owned values
  created inside the loop are dropped at the end of each iteration.
- **Single assignment for Owned.** Reassigning an Owned binding is `E0284` for
  now; an Owned value is created once and then moved.

## Deterministic drop

Drops are inserted by the IR lowerer using the move information from
`lime_sema`. At every scope exit and before every `kembalikan` / `berhenti` /
`lanjut`, each still-owning Owned local in the relevant scopes is dropped in
reverse declaration order. A moved-out binding is skipped, and dropping a binding
marks it dropped, so no value is ever freed twice. A returned Owned value is
moved out first, so it is not dropped by the returning frame.

## Ownership, generics, and FFI

- **Generics.** Ownership is enforced per monomorphized instance, where each type
  parameter is concrete and its category is known; a generic body is checked for
  ownership once per instantiation.
- **FFI.** Owned types may not appear in an `ekstern` signature or be passed to
  an extern function (`E0283`): the C ABI has no knowledge of Lime ownership.

## References and borrowing

A reference is a native pointer to an existing value that does **not** own it.
`&T` is a shared (read-only) reference; `&ubah T` is a mutable reference. They
are created with the builtins `ambil(x)` and `ambil_ubah(x)`, read with `*r`,
and written (only through `&ubah`) with `*r = e`. A reference is Copy: borrowing
allocates nothing, drops nothing, and touches no reference count — the referent
is left exactly as it was.

### Aliasing rules

A *loan* is an outstanding borrow of a place. The checker keeps, per place:

- **Shared loans may coexist.** Any number of `&T` to the same value can be live
  at once, and the value may be read while they are.
- **A mutable loan is exclusive.** While a `&ubah T` is live, no other reference
  (shared or mutable) to that value may be created (`E0295`), and the value may
  not be read or moved through its owner.
- **No mutation through the owner while borrowed.** Assigning to a value (or a
  field/element of it) while any loan on it is live is `E0296`.

### Loan lifetime (lexical, non-escaping)

Loan extent is lexical and deliberately simple:

- A reference **bound with `nilai`** lives until the end of its enclosing block.
- A reference **created only as a call argument** lives just for that statement,
  so sequential borrows compose: `naik(ambil_ubah(n)) naik(ambil_ubah(n))` is two
  separate, non-overlapping mutable loans.

References are **non-escaping**: a reference may not be returned from a function,
stored in a struct or array literal, put in a `kotak`, or cross an FFI boundary
(`E0297`, and `E0298` at the FFI boundary). Because a reference can never leave
the scope in which its referent is live, it can never outlive that value — this
is what makes the model sound without lifetime parameters or annotations.

### Interaction with ownership

- **Borrowing never moves.** `ambil(x)` / `ambil_ubah(x)` leave `x` fully owned
  and usable after the borrow ends; borrowing a `kotak<T>` does not duplicate or
  transfer ownership.
- **A moved value cannot be borrowed.** Taking a reference to a moved-out binding
  is `E0280`.
- **Drops stay safe.** Non-escaping loans guarantee every reference is dead
  before its referent's drop point, so the deterministic-drop rules above are
  unchanged and no value is freed while a reference to it exists.

### Reading through references

`*r` copies `T` out of the reference, so it requires `T` to be Copy; reading an
Owned value out through a reference (e.g. `*b` for `b: &kotak<i64>`) is `E0293`.
Use `isi(b)` to read the Copy payload of a borrowed box instead.

### Generics and FFI

- **Generics.** References work in generic functions: `&T` / `&ubah T` parameters
  infer `T`, and the borrow and Copy-through-deref rules are checked per
  monomorphized instance, so `fungsi baca<T>(x: &T) -> T { kembalikan *x }` is
  accepted for a Copy `T` and rejected (`E0293`) when instantiated with an Owned
  `T`.
- **FFI.** References are a Lime-internal aliasing concept the C ABI cannot
  honor, so they are rejected at the boundary rather than silently reinterpreted
  as pointers or ownership (`E0298`).

### Lowering (zero cost)

`ambil`/`ambil_ubah` lower to the address of the referent's stack slot (or a
`getelementptr` into a field/element); `*r` lowers to a native `load`, and
`*r = e` to a native `store`. There is no wrapper, tag, or count — a reference is
exactly a machine pointer, and reference-only code performs no heap traffic.

### Results (`Hasil<T, R>`)

The recoverable-result type `Hasil<T, R>` (`docs/TYPE_SYSTEM.md`) is, for now,
restricted to Copy (`salinan`) payloads, so it is itself Copy: it lives inline as
a tagged aggregate `{ tag, T, R }`, is copied rather than moved, and is never
dropped. `sukses`/`gagal` construct it, `coba` unwraps or early-returns the
failure, and the inspection builtins read it — all with no allocation and no
drop obligations. Owned payloads (which would need the ok field dropped only on
success and the err field only on failure — a conditional drop) are a documented
later extension, rejected with a diagnostic today rather than mishandled.

### Dynamic lists (`daftar<T>`)

`daftar<T>` is the growable collection: an Owned value laid out as
`{ data: *T, len, cap }` that owns a heap buffer. It is built with `daftar()`,
grown with `dorong` (which `realloc`s, doubling capacity), read with `elemen`
and `panjang`, and written with `atur`; indices are runtime bounds-checked. Being
Owned, it moves rather than copies and is dropped deterministically — its drop
frees the buffer with `free`, so there is one `free` per list and no leak. For
now `T` must be Copy, so the buffer needs no per-element drop (the same rule
`Hasil` uses); Owned element types, which would require dropping each live
element before freeing the buffer, are a documented follow-up.

### Concurrency and data races

`mulai(f, n)` spawns a real OS thread (via `pthread`) running `f` with the Copy
argument `n`, and `tunggu(h)` joins it and returns the result. Safety is
structural, not lock-based: the argument is **moved in by value** and the result
comes back **through the join**, so a spawned thread shares no mutable state with
its parent — and Lime has no shared mutable globals, while references are
non-escaping and so cannot be captured into a thread. There is therefore no
data-race surface to guard. The current cut supports `angka -> angka` workers on
POSIX hosts; passing richer (Copy) payloads, channels for message passing, and
Windows threads are follow-ups, each gated behind a real design rather than a
lock bolted onto shared mutable memory.

### Method receivers

Trait and inherent methods (`docs/TYPE_SYSTEM.md`) take their receiver as the
first parameter, typed `&Diri`, `&ubah Diri`, or `Diri`. A method call reuses the
borrowing machinery above with no new rules: a `&Diri` receiver takes a shared
loan on the receiver place, `&ubah Diri` a mutable loan, and `Diri` moves it.
Fields are reached through the receiver by auto-dereferencing the reference
(`diri.x` compiles to a `getelementptr`/`load` through the pointer), and a
`&ubah Diri` method may write them in place. Methods therefore allocate nothing
and, being statically dispatched, lower to ordinary direct calls.

## Strings

Design target: `teks` is `{ ptr: bita*, len: u64 }`. Owned strings additionally
carry a capacity and a heap allocation; string literals are borrowed views into
static read-only data.

**Bootstrap interim:** string literals are emitted as NUL-terminated static
UTF-8 buffers and `teks` is `i8*`. This is memory-safe (all such strings are
immutable and static) but does not yet carry an explicit length. Length-carrying
fat pointers and owned strings are the next memory-model milestone. This
limitation is intentional and isolated to the string representation; it does not
affect the soundness of the numeric/control-flow core.

## Structs

Structs are value types with a real compile-time layout. Each `tipe` becomes an
LLVM aggregate (`{ f0, f1, ... }`); field order is preserved and the target C ABI
governs alignment and padding, so a Lime struct can cross the FFI boundary
predictably.

- **Binding / passing / returning** copies the aggregate value. There is no
  implicit boxing and no heap allocation for a struct itself.
- **Construction** lowers to `insertvalue` from `undef`; **field reads** lower to
  `extractvalue`; **field mutation** loads the root aggregate, applies a nested
  `insertvalue`, and stores it back. `llc -O2` (SROA / mem2reg) promotes these
  slots to registers, so simple struct code carries no memory traffic.
- **Mutability** is whole-value: mutating `p.posisi.y` requires the root binding
  `p` to be `ubah`. This keeps the ownership story simple — one owner per struct
  value — and sets up the borrow model below without partial-field aliasing.

A struct whose fields are all stack values is itself a stack value; a struct that
will later contain heap-owning fields (e.g. an owned `teks`) will follow the
deterministic-destruction rules once ownership lands.

## Arrays

Fixed-size arrays `[T; N]` are value types compiled to the LLVM array type
`[N x T]`. They live inline — in a stack slot for a local, or inside their
containing struct/array — with no header and no heap allocation. Assigning,
passing, or returning an array copies all `N` elements by value.

- **Indexing** computes an element address with `getelementptr` and then
  `load`/`store`s a single element, so element access does not copy the array.
- **Bounds checking**: each index is compared against `0` and the (compile-time
  constant) length before the access; an out-of-range index calls libc `abort`,
  turning a would-be memory-safety violation into a deterministic process abort.
  When the index is a constant the check folds away at `-O2`.
- **Iteration** (`untuk x pada xs`) copies each element into the loop binding;
  index-based iteration over `panjang(xs)` avoids the copy when the element type
  is large.

Growable collections (a `daftar<T>` vector) will be a standard-library type built
on explicit heap allocation, distinct from these inline fixed arrays; the array
primitive is the zero-overhead building block beneath them.

## No hidden allocations

The **only** source of heap allocation is an explicit Owned value (`kotak(...)`).
A program that uses no Owned type performs **zero** heap allocations at runtime —
it compiles to code that uses only the stack plus static data. When Owned values
are used, every `malloc` has a matching compiler-inserted `free` at the owner's
drop point; there are no hidden allocations and no leaks. `tampilkan` calls libc
`printf` with static format strings and stack/register arguments.

## Layout & ABI

Struct layout follows the target C ABI (field order preserved, natural
alignment, tail padding) so Lime structs can be passed across the FFI boundary
predictably. This is an ABI choice, not a semantic dependence on C.
