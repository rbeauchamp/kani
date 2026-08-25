<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Qualification profile: `core-v1`

Status: **draft**

Owner: **not yet assigned**

First candidate: `4f7baae414d596eaa82ee90ee529f9957ca565dd`

## Purpose

`core-v1` is a deliberately conservative first release envelope for explicitly
authored, bounded Kani proof harnesses. It does not attempt to qualify every
Kani capability or every Rust program.

Application-specific source, manifests, and evidence are not part of this public
repository. A public distribution receipt establishes only the profile and
public test corpus stated here. Each adopting application must maintain its own
exact, access-controlled acceptance receipt before relying on the distribution.

## Proposed qualified platforms

- `aarch64-apple-darwin` on the recorded macOS environment;
- `x86_64-unknown-linux-gnu` on the recorded Ubuntu environment.

Other upstream-built bundles may be useful but must not be described as
qualified until the same public profile gate runs on them.

## Included behavior

- explicitly named `#[kani::proof]` harnesses;
- scalar and profile-reviewed derived `kani::Arbitrary` values;
- `kani::any`, `kani::assume`, assertions, and declared `kani::cover!`
  reachability obligations;
- bounded loops with explicit unwind bounds and passing unwind assertions;
- default integer arithmetic, bounds, panic, division, shift, and pointer
  validity checks documented by Kani; and
- focused use of stable APIs exercised by the frozen public corpus.

Inclusion is conditional on exact harness replay and mutation success. This list
does not broaden Kani's documented guarantees.

## Explicit exclusions for the first release

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

The receipt must confirm these provisional identities with exact observations:

| Component | Required value |
|---|---|
| Kani base | `4f7baae414d596eaa82ee90ee529f9957ca565dd` |
| Downstream patches | Empty at baseline; re-record at candidate freeze |
| Rust | `nightly-2026-04-01` |
| CBMC | `6.10.0` |
| Kissat | `4.0.1` |
| Charon | `b250680abd40ff1aaa07081d0497dc2755ed112e` |
| Cargo lock | SHA-256 `086f3cb1e49b5f54f8d2684d89f6fa17c32226202f77fe3c2f5f46205c28589f` |
| Build environments | Local macOS identity recorded; Linux label `ubuntu-22.04`, exact image version pending first receipt |

The runtime versions must be checked before proof execution.

The first release qualifies only the pinned default CBMC/Kissat execution path.
Another solver or concurrent result path requires its own profile and receipts.

The residual TCB includes rustc and its MIR, Kani's translation and models,
Irep/GOTO serialization, linking, CBMC, the selected solver, platform semantics,
the public harnesses, and their assumptions. The first release narrows and tests
that boundary; it does not prove it correct.

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

`core-v1` does not use Kani as the sole oracle for properties outside its model.
Unsafe or aliasing-sensitive code should use Miri and appropriate runtime tools;
unbounded mathematical claims should use a suitable proof system; ordinary
tests, fuzzing, sanitizers, and review remain required where applicable.

## Invalidation

The profile is invalidated by any change to the Kani candidate, ordered patch
set, toolchain tuple, target, build environment, flags, public corpus, harness
manifest, assumptions, or mutation set. Requalification creates a new receipt.
