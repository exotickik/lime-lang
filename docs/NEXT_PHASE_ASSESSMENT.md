# Phase 15 (Concurrency) — Design Assessment

Baseline: `main` after first-class function values — 235 tests, 0 warnings.

## Goal

A real, data-race-free concurrency primitive that fits Lime's ownership model —
actual OS threads, no faked parallelism, no lock bolted onto shared mutable
memory.

## Dependency handled first

Spawning a thread needs to hand a function to the runtime, which required
**first-class function values** (`fungsi(A) -> R`). That was implemented and
validated as its own step (functions passed/stored/returned/called indirectly),
and is also the prerequisite for FFI callbacks.

## Model

- `mulai(f, n)` runs `f` (a `fungsi(angka) -> angka`) on a new OS thread with the
  Copy argument `n`, returning a handle `Utas<angka>`.
- `tunggu(h)` joins the thread and returns its result.

Safety is **structural**: the argument is moved in by value and the result
returns through the join, so a thread shares no mutable state with its parent.
Lime has no shared mutable globals, and references are non-escaping (they cannot
be captured into a thread), so there is no data-race surface — the guarantee
comes from the ownership rules, not a mutex.

## Implementation

- New `Type::Utas(R)` (thread handle) and builtins `mulai` / `tunggu`, checked in
  sema (`mulai` requires an `angka -> angka` function and an `angka` argument;
  `tunggu` requires an `Utas`), diagnostics `E02D0`.
- Lowering uses libc `pthread` directly. The key insight: on x86-64 an
  `i64(i64)` function is ABI-compatible with pthread's `void*(void*)` start
  routine, so the Lime worker is passed **straight** to `pthread_create` and the
  `angka` argument straight as the `void*` — **no trampoline and no boxing**. The
  handle is the `pthread_t`; `tunggu` calls `pthread_join` and reads the returned
  value. `Utas<R>` lowers to `i64`.
- Codegen declares `pthread_create` / `pthread_join`; on modern glibc `pthread`
  lives in libc, so no extra link flag is needed on the host.

## Validation

Multi-thread programs compute deterministic results across threads and join them
(`examples/utas.lime`); CLI and sema tests cover spawning, joining, many threads,
and the misuse diagnostics; the IR shows real `pthread_create`/`pthread_join`.

## Non-goals this phase (documented, not faked)

- Richer (Copy) payloads and multiple arguments — need boxing / a small
  trampoline, deferred rather than faked.
- Channels / message passing and atomics — need shared, synchronized runtime
  state, taken up with a proper design later.
- Windows threads (`CreateThread`) — part of the native-platform milestone;
  the current primitive targets POSIX hosts.
