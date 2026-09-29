# Lime Memory Model

Lime targets performance-sensitive native software. It has **no garbage
collector** and no hidden background runtime. Memory management is deterministic
and, where possible, resolved at compile time.

## Principles

1. **Value semantics by default.** Primitives and small structs are values.
   Assignment and argument passing copy the value unless a reference is taken.
2. **Stack-first.** Locals live on the stack. The compiler places a value on the
   stack whenever escape analysis (planned) proves it does not outlive its frame.
   The bootstrap places all scalars and fixed-size locals on the stack.
3. **Explicit heap.** Heap allocation is never implicit. Growable containers and
   owned strings allocate through explicit standard-library APIs.
4. **Deterministic destruction (planned).** Owned resources are released at the
   end of their scope (RAII-style `hancurkan`/drop), in reverse construction
   order. No finalizer thread, no nondeterminism.
5. **Ownership & borrowing (planned).** Each owned value has a single owner.
   Borrows (`&`) are non-owning views checked so they never outlive the owner.
   Moves transfer ownership and invalidate the source binding.

## Ownership model (design, phased)

| Concept | Keyword/notation | Status |
|---|---|---|
| Owned value | default | value copies today; move semantics planned |
| Shared borrow | `&T` | planned |
| Mutable borrow | `&ubah T` | planned |
| Move | assignment of a non-`Copy` type | planned |
| Drop | `fungsi hancurkan(sendiri)` | planned |

The borrow checker is specified to run in `lime_sema` after type checking,
operating on the same typed AST, before IR lowering. Until it lands, the
bootstrap only compiles programs whose values are trivially copyable
(primitives, and structs of primitives), so no unsound aliasing is possible.

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

The bootstrap performs **zero** heap allocations at runtime for the currently
supported language subset: every program compiles to code that uses only the
stack plus static data. `tampilkan` calls libc `printf` with static format
strings and stack/register arguments.

## Layout & ABI

Struct layout follows the target C ABI (field order preserved, natural
alignment, tail padding) so Lime structs can be passed across the FFI boundary
predictably. This is an ABI choice, not a semantic dependence on C.
