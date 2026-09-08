<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Qualification profile: `core-v2`

Status: **draft**

Owner: **not yet assigned**

First candidate: recorded at candidate freeze; see the release manifest.

Successor to: [`core-v1`](core-v1.md), which remains immutable history together
with the `qualified-0.67.0+20260825.1` receipt. See
[ADR 0003](../decisions/0003-core-v2-on-stable-rust-1.97.1.md).

## Purpose

`core-v2` is the same deliberately conservative release envelope as `core-v1`
— explicitly authored, bounded Kani proof harnesses — now bound to the exact
stable Rust 1.97.1 toolchain tuple and to a frozen public proof corpus. It does
not attempt to qualify every Kani capability or every Rust program.

Application-specific source, manifests, and evidence are not part of this public
repository. A public distribution receipt establishes only the profile and
public test corpus stated here. Each adopting application must maintain its own
exact, access-controlled acceptance receipt before relying on the distribution.

## Qualified platforms

- `aarch64-apple-darwin` on the recorded macOS environment;
- `x86_64-unknown-linux-gnu` on the recorded Ubuntu environment.

Other bundles produced by the same candidate build may be useful but must not
be described as qualified until the same public profile gate runs on them. In
particular, the `aarch64-unknown-linux-gnu` bundle is published, if at all,
only with an explicit not-qualified label.

## Included behavior

- explicitly named `#[kani::proof]` harnesses;
- scalar and profile-reviewed derived `kani::Arbitrary` values;
- `kani::any`, `kani::assume`, assertions, and declared `kani::cover!`
  reachability obligations;
- bounded loops with explicit unwind bounds and passing unwind assertions;
- default integer arithmetic, bounds, panic, division, shift, and pointer
  validity checks documented by Kani; and
- focused use of stable APIs exercised by the frozen public corpus, including
  the Rust 1.96–1.97.1 stabilized items selected in the
  [compatibility ledger](../compat-ledger-rust-1.97.1.md).

Inclusion is conditional on exact harness replay and mutation success. This list
does not broaden Kani's documented guarantees.

## Explicit exclusions

Inherited from `core-v1` without broadening. Any future removal requires
direct evidence and review recorded in a superseding profile:

- autoharness;
- experimental quantifiers;
- nondeterministic or autoharness-generated `Rc<T>` and `Arc<T>` values;
- `--quiet` and its short form;
- `--export-json` as a verdict source;
- `--fail-fast`;
- parallel harness jobs and concurrent gates sharing a build directory;
- experimental loop contracts and synthesized contracts;
- `--restrict-vtable`;
- concurrency and data-race claims;
- non-default SMT solver backends;
- inline or global assembly;
- source-based code-coverage claims;
- dynamic trait-object dispatch and vtable-semantic claims;
- FFI/ABI claims not replaced by a reviewed model;
- pointer-aliasing claims under Stacked Borrows or Tree Borrows;
- pointer claims that depend on exact wrapped-address equality, ordering, or
  provenance after wrapping out of range;
- claims based on unspecified `repr(Rust)` layout, padding, field order, or
  transmute compatibility;
- custom symbol overriding or duplicate/missing-symbol behavior;
- experimental uninitialized-memory and valid-value checks;
- claims requiring unbounded proof; and
- any unreviewed or unledgered warning or unsupported operation,
  `UNDETERMINED`, unexplained `UNREACHABLE`, timeout, solver error, or missing
  expected harness.

## Required toolchain tuple

The receipt must confirm these identities with exact observations:

| Component | Required value |
|---|---|
| Kani base | Frozen candidate commit; recorded in the release manifest |
| Downstream patches | The ordered merged PR series #12, #13, #14, #15 on upstream `b07abe8a72f8eb1ef1ea3521ca6b00a973d341bc`; re-recorded at freeze |
| Rust channel | `1.97.1` (exact stable; a later nightly is not a substitute) |
| `rustc` | `rustc 1.97.1 (8bab26f4f 2026-07-14)`, commit `8bab26f4f68e0e26f0bb7960be334d5b520ea452` |
| Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| LLVM | `22.1.6` |
| Required components | `llvm-tools`, `rustc-dev`, `rust-src`, `rustfmt` |
| CBMC | `6.11.0` |
| Kissat | `4.0.1` |
| Charon | `b250680abd40ff1aaa07081d0497dc2755ed112e` (experimental LLBC back-end; not part of the qualified path) |
| Cargo lock | SHA-256 recorded at candidate freeze |
| Build environments | Local macOS identity recorded; Linux label `ubuntu-24.04`, exact image version recorded in the first receipt |

The runtime versions must be checked before proof execution. Release-mode
`cargo kani` must use the bundled exact Rust 1.97.1 Cargo, compiler, sources,
libraries, and tools; the install path fails closed on any version mismatch
(the release workflow's TestLocalToolchain gate).

This release qualifies only the pinned default CBMC/Kissat execution path.
Another solver or concurrent result path requires its own profile and receipts.

The residual TCB includes rustc and its MIR, Kani's translation and models,
Irep/GOTO serialization, linking, CBMC, the selected solver, platform
semantics, the release workflow that produced the bundles, the public
harnesses, and their assumptions. This release narrows and tests that boundary;
it does not prove it correct.

## Public corpus

The frozen public corpus lives under
[`qualification/fixtures/public-corpus`](../fixtures/public-corpus/) and
consists of generic, application-neutral proof harnesses with declared cover
obligations and a recorded unreachable-check distribution. The consumer
manifest under [`qualification/manifests/core-v2`](../manifests/core-v2/)
binds the corpus checkout identity, the exact expected harness set, cover
counts, unreachable counts, and the reviewed diagnostic ledger.

## Result policy

Promotion requires:

- exact equality between expected and enumerated public harness sets;
- a successful result for every expected assertion and unwinding assertion;
- satisfaction of every required cover obligation;
- zero `UNDETERMINED` results;
- an explanation and explicit expectation for every `UNREACHABLE` result;
- an exact diagnostic ledger whose entries record origin, affected harnesses,
  reachability, and release impact, with rejection of new or changed text;
- nonzero exit for every red mutation and every deliberately broken harness;
- detection of zero-harness filters and partial execution; and
- independent parsing/checking of terminal output until a structured result
  protocol is separately qualified.

## Complementary evidence

`core-v2` does not use Kani as the sole oracle for properties outside its
model. Unsafe or aliasing-sensitive code should use Miri and appropriate
runtime tools; unbounded mathematical claims should use a suitable proof
system; ordinary tests, fuzzing, sanitizers, and review remain required where
applicable. The Rust 1.96–1.97.1 compatibility corpus (merged in PR #15) is
regression evidence for the toolchain migration; it is not itself a qualified
claim beyond what the frozen public corpus replays.

## Invalidation

The profile is invalidated by any change to the Kani candidate, ordered patch
set, toolchain tuple, target, build environment, flags, public corpus, harness
manifest, assumptions, or mutation set. Requalification creates a new receipt.
