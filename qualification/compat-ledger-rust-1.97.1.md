<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Rust 1.96–1.97.1 compatibility classification ledger

Status: **active**

This ledger is the classification record for the Rust 1.97.1 compatibility
corpus required by issue #1 Phase 2. The previous toolchain (a May 2026
nightly) predates the final Rust 1.96 and 1.97 releases, so both release
deltas were reviewed against the official release notes and announcements:

- Rust 1.96.0 (2026-05-28): [blog](https://blog.rust-lang.org/2026/05/28/Rust-1.96.0/), [release notes](https://releases.rs/docs/1.96.0/)
- Rust 1.96.1 (2026-06-30): [release notes](https://releases.rs/docs/1.96.1/)
- Rust 1.97.0 (2026-07-09): [blog](https://blog.rust-lang.org/2026/07/09/Rust-1.97.0/), [release notes](https://releases.rs/docs/1.97.0/)
- Rust 1.97.1 (2026-07-16): [blog](https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/), [release notes](https://releases.rs/docs/1.97.1/)
- [RELEASES.md](https://doc.rust-lang.org/releases.html) for the full stabilized-API lists.

Classification tags:

- **[V] verified by Kani** — a proof harness in the corpus exercises the change;
- **[C] compile-only** — Kani compiles (or rejects) the construct; verification
  content is trivial or the change is syntactic/lint-level;
- **[N/A] not applicable** — no Kani-observable behavior (e.g. rustdoc, other
  targets, Cargo plumbing);
- **[OUT] outside profile** — real behavior change that falls under the
  qualification profile's stated exclusions (concurrency, linkage attributes,
  unwinding, …), recorded here rather than silently dropped.

Empirical claims below were confirmed against the exact stable 1.97.1
toolchain (`rustc 1.97.1 (8bab26f4f 2026-07-14)`) on `aarch64-apple-darwin`,
including `rustc --print cfg` for both qualification targets and direct
compile/verify runs of every corpus case.

## Corpus case inventory

### Verified by Kani (`tests/kani/Compat196/`, `tests/kani/Compat197/`)

| Case | Source change | Corpus file |
|---|---|---|
| Integer `highest_one`/`lowest_one`/`isolate_highest_one`/`isolate_lowest_one`/`bit_width` + `NonZero` variants, symbolic vs relational specs; concrete edges | 1.97.0 stabilized APIs | `Compat197/bit_ops.rs` |
| `core::range::{Range, RangeFrom, RangeToInclusive}` (`Copy` + `IntoIterator`), iterators, legacy conversions, symbolic-bounds sequence proof | 1.96.0 RFC 3550 range types | `Compat196/core_range.rs` |
| `core::assert_matches!`/`debug_assert_matches!` positive + `should_panic` negative | 1.96.0 stabilized macros | `Compat196/assert_matches.rs` |
| `NonZero` unsigned range iteration (`..`, `..=`, `next_back`) | 1.96.0 library change | `Compat196/nonzero_range.rs` |
| `From<T> for LazyCell` / `From<T> for LazyLock` (initialized fast path) | 1.96.0 stabilized impls | `Compat196/lazy_cell.rs` |
| `#[repr(Int)]` enums with uninhabited-ZST fields: `size_of`/`align_of`/`size_of_val` pinned | 1.96.0 layout fix | `Compat196/repr_int_layout.rs` |
| `cfg(target_has_atomic_primitive_alignment)` values, `AtomicU64` vs `u64` alignment, per-arch `"128"` differential | 1.97.0 stabilized cfg ([rust#155006](https://github.com/rust-lang/rust/pull/155006)) | `Compat197/atomic_primitive_alignment.rs` |
| `Default for std::iter::RepeatN` (empty default, equivalence with `repeat_n(v, 0)`) | 1.97.0 stabilized impl | `Compat197/repeat_n.rs` |
| const-stable `char::is_control` (const table + symbolic char vs reference range) | 1.97.0 const-stabilization | `Compat197/char_is_control.rs` |
| Import grammar acceptance: `use m::self as mm;`, `use E::self as EE;` | 1.97.0 trailing-`self` imports ([rust#155137](https://github.com/rust-lang/rust/pull/155137)) | `Compat197/import_self.rs` |

### Compile-only rejections (`tests/ui/compat196/`, `tests/ui/compat197/`)

| Case | 1.97.1 behavior (verified) | Corpus dir |
|---|---|---|
| `use S::{self as Other};` for struct `S` | `error[E0432]: unresolved import` ("`S` is a struct, not a module") | `ui/compat196/struct-self-import` |
| Tuple-index shorthand in struct patterns (`let P { 0 } = p;`) | parse error "expected identifier, found `0`" | `ui/compat197/tuple-index-pattern` |
| Generic args on a module path segment (`m::<u8>::A(1)`) | `error[E0109]: type arguments are not allowed on module` | `ui/compat197/module-segment-generics` |
| `static` of uninhabited type | `error: static of uninhabited type` (deny-by-default `uninhabited_static`; still future-incompatible lint, not yet hard error) | `ui/compat196/uninhabited-static` |
| `#[export_name = ""]` | `error: \`export_name\` may not be empty` | `ui/compat197/empty-export-name` |

### Dependency-resolution cases (`tests/cargo-kani/`)

| Fixture | `rust-version` | Expected |
|---|---|---|
| `rust-version-below` | `1.96.0` | verifies |
| `rust-version-at` | `1.97.1` | verifies |
| `rust-version-above` | `1.98.0` | predictable rejection: `error: rustc 1.97.1 is not supported by the following package: … requires rustc 1.98.0` at build-plan time, before any compilation |

## Corrections to naive release-note readings

These were established empirically during corpus authoring; they matter for
anyone repeating this exercise:

1. **Import grammar**: the 1.97.0 acceptance is `use`-position only.
   `type X = m::self;` is still rejected (E0573). The struct-parent rejection
   from 1.96.0 reports as E0432, not a dedicated message.
2. **Module-segment generics**: turbofish on the *final* segment
   (`m::A::<u8>`) remains accepted; the 1.97.0 change ([rust#154962](https://github.com/rust-lang/rust/pull/154962))
   forbids generics on the *module* segment (`m::<u8>::A`).
3. **`uninhabited_static`** is a deny-by-default future-incompatible lint on
   1.97.1 (compilation stops, but the diagnostic notes a future hard error;
   the corpus pins both lines so the conversion is flagged). The stable
   trigger is an `extern "C"` static; an initializer expression fails const
   evaluation first instead.
4. **Signed integers have no `bit_width`** (`uint_bit_width` is unsigned-only);
   signed `highest_one`/`lowest_one`/`isolate_*` operate on the `as unsigned`
   bit pattern.
5. **`core::range`**: no `RangeToInclusiveIter` exists (unbounded below);
   `RangeIter`'s `ExactSizeIterator` covers only `usize/u8/u16/isize/i8/i16`.
6. **`LazyCell::from(v)` needs an explicit type annotation** (E0283: the
   `F = fn() -> T` default does not apply during method-call inference).
7. **Step for `NonZero` is unsigned-only**, so `NonZero` range iteration is
   an unsigned-only feature.
8. **`!` enum fields remain feature-gated** on stable; the corpus uses an
   empty enum as the uninhabited ZST.
9. **`cfg(target_has_atomic_primitive_alignment)` value sets are
   target-differential**: `8,16,32,64,128,ptr` on `aarch64-apple-darwin`;
   `8,16,32,64,ptr` (no `"128"`) on `x86_64-unknown-linux-gnu` — both
   confirmed via `rustc --print cfg` on 1.97.1.

## Classified, no corpus case

### [N/A] not applicable to Kani

- 1.96.0: LoongArch link relaxation; `riscv64gc-unknown-fuchsia` RVA22; SGX
  fix; wasm `--allow-undefined` removal; AVR `c_double`; JSON target-spec
  internals; `#![reexport_test_harness_main]` gating; `-Csoft-float` removal;
  external LLVM ≥ 21; inference-guidance diagnostics; pointer-validity docs
  refactor; never-type tuple coercion guidance.
- 1.96.1: Cargo timeout/retry fix; libssh2 CVE fixes. The rustc
  MIR-optimization miscompilation fix ([rust#158214](https://github.com/rust-lang/rust/pull/158214))
  is a *corpus note*: expectations are pinned to 1.96.1+/1.97.1 behavior.
- 1.97.0: LoongArch target features (`div32`, `lam-bh`, `lamcas`, `ld-seq-sa`,
  `scq` — not aarch64/x86); nvptx64 ISA drop; `dead_code_pub_in_binary` lint
  (allow-by-default); v0-mangling-by-default and `linker_messages` lint
  (recorded as qualification notes for kani-compiler/kani-driver internals,
  not corpus cases); rustdoc `--emit`/`--remap-path-prefix`; UEFI `File: Send`;
  Windows socket `BrokenPipe` change; `must_use` equivalence for
  `Result<T, !>`/`ControlFlow<!, T>` (lint-only); float-fallback future-compat.
- 1.97.1: LLVM-optimization miscompilation fix ([rust#159035](https://github.com/rust-lang/rust/pull/159035));
  *corpus note*: expected outputs are pinned to 1.97.1, never 1.97.0.
- Cargo (all four versions): `git`+`registry` dependencies; target
  `rustdocflags`; `build.warnings`; `resolver.lockfile-path` (the only
  lockfile-adjacent change — a config path addition); `cargo clean`
  target-dir guard; `-m` shorthand; CVE-2026-5222/5223. **Explicit negative:
  no `rust-version` handling, resolver-semantics, or lockfile-format changes
  in 1.96–1.97.**

### [OUT] outside the qualification profile

- 1.96.0: s390x vector inline-asm registers; `From<T> for AssertUnwindSafe` /
  `LazyLock` (beyond the initialized fast path — thread synchronization);
  `BTreeMap::append` panic behavior; `export_name`/`link_name`/`link_section`
  first-wins (linkage attributes); `Pin` unsize-coercion rejection and
  RPITIT privacy hard error (covered conceptually by rejection tests only if
  they resurface); ManuallyDrop const patterns; `expr`-metavariable-into-`cfg`.
- 1.97.0: mach-O `link_section` validation; `#[link_name]`/`#[link]`
  validation; `varargs_without_pattern` in dependencies; `repr(Rust)` enum
  encoding change ([rust#155473](https://github.com/rust-lang/rust/pull/155473))
  — Kani takes layouts from rustc, recorded as a qualification note to
  recompute rather than assume any Kani-side layout expectation; `pin!`
  deref-coercion fix; `std::char` module deprecations and removed hidden
  `f64` methods (deprecation/removal diagnostics only).

## Toolchain-orchestration checks

Required by issue #1 Phase 2 and covered as follows:

- `cargo kani` metadata/build orchestration and the rust-version gate are
  exercised by the three `cargo-kani` fixtures above; the above-target case
  shows cargo's rust-version check fires at build-plan time and kani-driver
  forwards it verbatim (exit 1).
- Bundled-toolchain identity (`cargo 1.97.1 (c980f4866 2026-06-30)`,
  `rustc 1.97.1 (8bab26f4f 2026-07-14)`) is enforced fail-closed by the
  release workflow's TestLocalToolchain gate (merged in PR #14) and will be
  re-bound to the frozen candidate in the Phase 4 profile.
- Same-source/lockfile comparison between ordinary stable 1.97.1 and the
  release bundle with isolated target directories is a Phase 4/5 activity
  against the built artifacts; it is recorded here so the profile binds it.

## Unverified residual

- The exact mechanics of 1.96.0 "allow passing `expr` metavariable to `cfg`"
  are taken from release-note wording only; no corpus case depends on it.
