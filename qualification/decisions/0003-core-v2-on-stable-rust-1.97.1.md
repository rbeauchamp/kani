<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# ADR 0003: `core-v2` profile and public corpus on stable Rust 1.97.1

Status: accepted
Date: 2026-09-08

Supersedes: nothing. [`core-v1`](../profiles/core-v1.md) and the
`qualified-0.67.0+20260825.1` receipt remain immutable history.

## Context

Issue #1 requires a new qualified prerelease built with the exact stable Rust
1.97.1 toolchain. The compiler migration (PR #14) and the Rust 1.96–1.97.1
compatibility corpus (PR #15) are merged. Three structural differences from the
`core-v1` cycle drive new decisions:

1. **No upstream artifacts exist for this tuple.** `core-v1` repackaged
   upstream-built bundles byte-for-byte. Upstream builds nightlies only, so the
   1.97.1 artifacts must come from the fork's own release workflow, which #14
   generalized for exact stable toolchains (channel-aware archive resolution,
   official `.sha256` sidecar verification, rustc-version equality gate).
2. **`core-v1` shipped without a public proof corpus.** Its public evidence was
   bootstrap-grade by design ("a public corpus, SBOM, and independent public
   review is not complete"). Issue #1 Phase 4 makes a frozen public corpus,
   mutation set, and receipts mandatory.
3. **The gate hardcoded the `core-v1` profile name.** A versioned successor
   needs the gate to take the profile identity from the manifests while
   continuing to validate the unchanged `core-v1` manifests.

## Decision

1. **Create `core-v2` as a new profile document**; do not edit `core-v1`.
   `core-v2` preserves every `core-v1` exclusion verbatim and changes only what
   the new toolchain and corpus require.
2. **Build release artifacts from the fork's release workflow** at the frozen
   candidate commit on all four supported runners (macOS ARM64, Ubuntu 22.04
   x86_64, Ubuntu 24.04 x86_64, Ubuntu 24.04 ARM64). The `qualified` label
   applies only to targets that pass the frozen profile gate from the built
   artifacts: `aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`. The
   `aarch64-unknown-linux-gnu` bundle is published only if it is explicitly
   labeled as not qualified.
3. **Author a public corpus crate** under `qualification/fixtures/` with
   generic, application-neutral proof harnesses, declared cover obligations,
   and a recorded unreachable-check distribution. The consumer manifest under
   `qualification/manifests/core-v2/` binds the corpus checkout identity,
   expected harness set, covers, unreachable counts, and diagnostic ledger.
4. **Generalize the gate** (`qualification/scripts/core_v1_gate.py`) to read the
   profile name from the manifests and require toolchain/consumer agreement,
   keeping schema 1 and the existing `core-v1` manifests valid. The script keeps
   its filename for receipt continuity.
5. **Run the frozen profile gate from the built artifacts** on macOS ARM64
   (local, isolated environment) and on Ubuntu 24.04 x86_64 (CI job bound to
   the exact candidate and artifact hashes), committing both receipts under
   `qualification/receipts/`.
6. **Freeze the candidate as the merged `main` commit** that contains the
   profile, corpus, and gate changes; manifests recording artifact hashes are
   committed afterwards as immutable receipts referencing that commit, matching
   the `core-v1` record-keeping order (candidate first, receipts after).

## Consequences

- The release workflow becomes part of the qualified TCB boundary description;
  its stable-toolchain path was regression-tested in PR #14's
  TestLocalToolchain jobs on all four runners.
- Local gate iteration can use the development build (`target/kani`) before
  freeze; bootstrap runs are not final evidence (issue #1 Phase 4).
- Any change to the corpus, manifests, gate, or toolchain tuple after freeze
  invalidates the receipts and requires requalification.
