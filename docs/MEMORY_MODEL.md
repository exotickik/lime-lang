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
  (`E0280`, *nilai digunakan setelah dipindahkan*).

## `kotak<T>` — the owned heap cell

`kotak<T>` is the primitive Owned type: a single `T` on the heap, owned by
exactly one binding.

- `kotak(x)` allocates (libc `malloc`), moves `x` in, and yields a `kotak<T>`.
- `isi(k)` reads a **copy** of the contents (requires `T` to be Copy); it borrows
  `k` without consuming it — the read path while first-class references are
  pending (Phase 9).
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

## References (Phase 9)

First-class borrows (`&T` shared, `&ubah T` mutable) with aliasing rules are a
separate subsystem and are scheduled for Phase 9. The Phase 8 model is coherent
without them: Owned values move, are read with `isi`, and are handed back by
returning them. The planned reference rules — a shared borrow may alias freely
but not coexist with a mutable borrow, a mutable borrow is unique, and no borrow
may outlive its owner — will be checked in `lime_sema` on the same typed AST.

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
