<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Roadmap

Goal: publish the first stable qualified downstream release as soon as the
restricted `core-v1` profile has complete exact-head evidence. Speed comes from
narrowing the envelope and reusing exact upstream evidence, not from weakening
the promotion gate.

This file owns live task status and next action.

## Current status

| Item | State | Evidence or blocker |
|---|---|---|
| Public fork | Complete | `https://github.com/rbeauchamp/kani` |
| Upstream baseline | Frozen for assessment | `4f7baae414d596eaa82ee90ee529f9957ca565dd` |
| Research and operating model | In progress | Bootstrap branch documents require final publication review |
| `core-v1` profile | Draft | Public corpus and remaining mutations not frozen |
| Controlled-use prerelease | Published | [`0.67.0+qualified.20260825.1`](https://github.com/rbeauchamp/kani/releases/tag/qualified-0.67.0%2B20260825.1); signed tag, hashes, provenance, and final download readback complete |
| Candidate qualification | In progress | Controlled-use artifact transport complete; broader public promotion gates remain |
| Stable release | Blocked | Promotion gate incomplete |

## Assumption verdict

The assumption that post-0.67.0 development produced useful releasable work is
**supported but not yet promotion-complete**. The candidate contains substantive
correctness fixes, a newer toolchain, and successful upstream release-bundle
tests. The stronger assumption that current `main` is uniformly safer or ready
for unrestricted use is **not supported** because known result-integrity and
soundness items intersect newer or optional features.

The fastest defensible route is a stable release qualified only for `core-v1`,
with risky surfaces excluded or made fail-closed.

## Phase 0 — bootstrap

- [x] Re-authenticate GitHub CLI as the intended account.
- [x] Create `rbeauchamp/kani` as a public fork of `model-checking/kani`.
- [x] Clone and verify `origin`, `upstream`, `main`, clean status, and exact
      baseline SHA.
- [x] Capture the initial research, decision, operating model, profile, and
      candidate record.
- [x] Review public wording for technical accuracy, gratitude, neutrality,
      confidentiality, and absence of implied upstream endorsement.
- [x] Commit and push the bootstrap branch after local checks.

Exit gate: public documentation is internally consistent, respectful, linked,
confidentiality-safe, and bound to the recorded baseline.

## Phase 1 — freeze the first public profile corpus

- [ ] Define the complete public harness set, exact names, counts, flags,
      unwind policy, assumptions, stubs, covers, and expected diagnostics.
- [ ] Classify every used Kani and Rust feature against `core-v1`.
- [ ] Remove or mechanically block every excluded feature from the release
      claim.
- [ ] Define red mutations for false assertions, missing harnesses, vacuity,
      inadequate unwind, bad runtime CBMC, failed or partial execution, and
      result parser behavior.
- [ ] Freeze qualified platform and build-environment identities.

Application-specific acceptance remains in the adopting application's own
access-controlled system and cannot broaden the public release claim.

Exit gate: `core-v1` has no unresolved value that can change proof meaning.

## Phase 2 — disposition known candidate hazards

- [ ] Reproduce or inspect [PR #4719](https://github.com/model-checking/kani/pull/4719)
      at its exact head; either integrate an independently reviewed fail-closed
      fix or keep quantifiers mechanically excluded.
- [ ] Reproduce or inspect [issue #4752](https://github.com/model-checking/kani/issues/4752)
      and mechanically exclude affected smart-pointer generation.
- [x] Add a gate that rejects `--quiet`, validates runtime CBMC 6.10.0, and
      verifies the expected harness set independently.
- [x] Ensure `--export-json` is not consumed as a verdict oracle while
      [issue #4731](https://github.com/model-checking/kani/issues/4731) remains
      unresolved.
- [x] Investigate exact compiler-performance run
      [32815085770](https://github.com/model-checking/kani/actions/runs/32815085770)
      and record its release impact.
- [x] Triage every currently open `[F] Soundness` item against `core-v1` and
      record exclusions and residual TCB boundaries in
      [`2026-08-25-open-soundness-triage.md`](evidence/2026-08-25-open-soundness-triage.md).
- [x] Run an initial targeted search for unlabeled false-success and
      result-integrity reports and record controls in
      [`2026-08-25-result-integrity-triage.md`](evidence/2026-08-25-result-integrity-triage.md).
- [ ] Search all open P0/P1 items for intersections with `core-v1`, including
      unlabeled issues.

The compiler-performance job built and timed both revisions successfully, then
identified a potential `s2n-codec` compile-time regression from about 11.77 to
13.97 seconds in the IQR averages. This is P2 performance evidence, not evidence
of a wrong verification verdict. It remains relevant to timeout budgets.

Exit gate: every known intersecting P0/P1 item is fixed, mechanically excluded,
or blocks the release. Exceptions cannot permit a false success.

## Phase 3 — build and qualify candidate 1

- [ ] Freeze `candidate/<version>` from the selected upstream SHA and ordered
      patch queue.
- [x] Reconcile exact upstream CI and release-bundle jobs for that SHA.
- [ ] Run the complete applicable Kani regression suite at the frozen head.
- [ ] Freeze and replay the complete public profile corpus on macOS ARM.
- [ ] Build or retrieve and qualify the exact Linux x86_64 artifact.
- [ ] Run every red mutation and require each one to be detected.
- [ ] Obtain an independent full-path review of the exact candidate.
- [ ] Generate the immutable qualification receipt, SBOMs, hashes, and
      signatures or attestations.

Exit gate: all required gates are terminal green at one exact commit and
artifact set, with no unexplained or missing result.

## Phase 4 — publish the first stable qualified release

The controlled-use prerelease is complete without claiming this stable-release
exit gate:

- [x] Publish exact upstream-built macOS ARM64 and Linux x86_64 bundles under a
      non-conflicting qualified tag.
- [x] Publish respectful release notes, the exact base, profile limitations,
      provenance, signed checksums, and installation instructions.
- [x] Verify every draft asset by clean download before publication.
- [x] Publish the signed annotated tag and GitHub prerelease.
- [x] Read back the public tag target, signature status, release state, asset
      names, sizes, GitHub digests, and every downloaded checksum.

- [ ] Validate downstream Cargo versioning and tag format against the release
      workflow. The provisional version is
      `0.67.0+qualified.YYYYMMDD.N`.
- [ ] Prepare release notes that thank upstream contributors, identify the exact
      upstream base, state that the distribution is unofficial, list the
      qualified profile and exclusions, and link the public receipt.
- [ ] Create a draft release and verify every artifact from a clean download.
- [ ] Obtain program-owner promotion approval.
- [ ] Publish the signed tag and release.
- [ ] Read back GitHub tag, commit, release state, artifact names, sizes, hashes,
      and attestations.
- [ ] Update adopting applications only after the readback succeeds and each
      application-specific acceptance gate passes in its own system.

Exit gate: the published release matches the qualified artifact set exactly and
can be rolled back to 0.67.0 or the previous qualified release.

## Phase 5 — upstream collaboration and recurring releases

- [ ] Submit each generally useful fix as a small upstream pull request following
      `CONTRIBUTING.md`.
- [ ] Offer reproducible reducers, regression tests, and review help without
      asking upstream to adopt downstream process or timelines.
- [ ] Remove downstream patches once upstream equivalents are merged and the
      new base is requalified.
- [ ] Propose release-process improvements upstream only when focused, useful
      independently, and welcome.
- [ ] Establish a predictable intake and qualification cadence based on
      downstream need.

## Full-fork escape hatch

A permanent semantic fork is not authorized by this roadmap. Propose it through
a superseding ADR only if measured collaboration and patch-queue evidence meet
the triggers in `operating-model.md` and sustainable independent ownership is
available.

## Immediate next action

Controlled adopters may pin the published tag and artifact hashes, run their
own access-controlled acceptance gate, and retain their receipt. In parallel,
finish the public fail-closed mutation corpus, freeze the smallest public
harness corpus, generate an SBOM, obtain independent public review, and replay
the Linux artifact on a recorded native x86_64 runner before stable promotion.
