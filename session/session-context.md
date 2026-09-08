<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Session context: complete the downstream Kani program

**Date:** 2026-09-07
**Source branch / starting basis:** `main` /
`934273e6d909be63df84a588a6514c7afaca0625` (merged PR #10).
**Persistence / resume authority:** the signed handoff commit on
`origin/codex/session-handoff`, containing this file. It is a later documentation
commit, not the starting-basis SHA above, and is not asserted to be merged into
`main`. Resolve its exact SHA with
`git log -1 --format=%H -- session/continue-session-prompt.md`.
**Owned surface:** this handoff branch; only the two `session/` files, ADR 0002,
and the dated infrastructure-review record. No pre-existing uncommitted work was
present. Issues #4 and #9 are the updated external planning surfaces.
**Active focus:** finish #4, then #5, then #1; afterward #7, justified #6 work,
and #8 once the artifact is qualified. Saving this session does not complete
that program or start its implementation.

## Authority and completed work

The [live roadmap #9](https://github.com/rbeauchamp/kani/issues/9) owns the
remaining mission and order; [project 4](https://github.com/users/rbeauchamp/projects/4)
owns status; each linked issue owns detailed acceptance criteria. The immediate
plan is [#4](https://github.com/rbeauchamp/kani/issues/4). Follow the authority
order in [`qualification/README.md`](../qualification/README.md); this handoff
does not replace the issue plans.

PR #10 is merged, #11 is closed, and both project entries are Done. Its focused
review and repairs, exact-head CI, merge/tree readback, and release preservation
are recorded in the
[infrastructure evidence](../qualification/evidence/2026-09-07-infrastructure-review.md).
Do not redo that review or reinterpret its self-test as a qualified release.

[ADR 0002](../qualification/decisions/0002-apple-silicon-macos-support.md) records
the approved Apple Silicon-only macOS scope and the nine required CI check types.
The Intel requirement was removed with explicit owner approval. The other
protections remain intact. Intel workflow jobs have **not** yet been removed;
that is part of #5. Linux coverage remains required. Asking how long Linux
checks take did not authorize removing them.

## Exact resume point

No upstream synchronization has been committed. #4's stale identities, allowed
downstream delta, and CBMC-warning claim have been reconciled in the live plan.
The conflict-free preview and its limits are in the dated evidence record.

Start with #4's Step 0: capture the handoff commit, verify a clean checkout,
fetch both remotes, and read back current `origin/main`, upstream commit/tree,
required checks, and protected release identity. Reconcile any drift in #4
before mutation. The first evidence gate is a pinned parent pair and a merge
preview that preserves the core-upstream and historical-release boundaries.

Preserve the handoff when branching from `origin/main`: if its commit is not
already on main, create the upstream merge from the pinned main basis first,
then carry this documentation commit onto the synchronization branch as directed
by #4. This keeps the upstream merge's two parents honest and the handoff files
available. Do not discard the saved branch until integration is verified.

## Standing constraints

- “An ounce of math is worth a pound of computation.” Identify semantics,
  assumptions, and invariants first; prefer construction, types, derived guards,
  and implementation-linked proofs. Preserve existing checks until replacement
  coverage is established. Scenario tests and sampled agreement do not prove
  universal correctness. Read the applicable `AGENTS.md` and operating model.
- “Preserve upstream ancestry with a merge commit.” #4 must use merge commits,
  including its GitHub PR merge; do not apply a default squash recipe to #4.
- “Keep the published prerelease's identities and assets unchanged.” Never
  rewrite its tag, manifests, historical evidence, or published files to match
  later development. New source/tool identities require new qualification.
- “Treat the proposed 15-minute turnaround as a measurement target until
  demonstrated.” No further required-CI reductions are authorized. Preserve
  functional/soundness coverage when relocating performance suites.
- #1 requires exact stable Rust 1.97.1 and its specified Cargo/LLVM tuple, not
  source compatibility under a later nightly. Reassess after #4; the 62-error
  count is stale. Retagging, layout, normalization, identity, publication, and
  installation obligations remain open.
- Keep all public artifacts generic to Kani, with upstream attribution and
  licensing. Private applications, local proprietary paths, credentials, and
  `.private-workspace/` contents must not enter commits or GitHub prose.
- Keep issues/project current. For completed work, verify required CI, the exact
  reviewed head, merge ancestry, issue closure, and owned-checkout cleanup. Do
  not close an issue merely because a narrower subtask passes.
- Preserve the full mission after #4: #5 → #1 → #7 → justified #6 → #8. Recheck
  speculative diagnoses and numerical claims in older issues before acting.

## Ruled out and local environment cautions

The dated evidence record and infrastructure contract preserve the rejected
receipt/result shortcuts and the runner's residual boundaries. In particular,
neither version warnings nor green development mutations establish pinned
qualification; arbitrary daemon containment is not claimed.

The pre-save checkout used the repository's May 1 nightly. The upstream target
uses `nightly-2026-08-21`, CBMC 6.11.0, and Kissat 4.0.1. That August nightly and
stable 1.97.1 were installed locally during preparation; installation did not
perform the compiler migration. Re-read repository pins and installed tools
after checkout. An existing local Kani distribution supplied CBMC 6.8.0 and is
not the pinned runtime for the new target. Never treat cached build binaries as
evidence that current sources were rebuilt.

The GNU `xargs -d` copyright script failed on macOS; an equivalent NUL-delimited
audit passed. Use an appropriate platform-equivalent check without dropping its
coverage. The local CI watcher was stopped after PR #10 merged; there is no
active local verification process to resume. GitHub job states are mutable and
must be read live if they affect a decision.

## Mission leverage

No separate mission-leverage card is used here. The live roadmap is the mission
authority. Immediate value is importing upstream correctness work with preserved
ancestry and qualification boundaries; a new qualified stable-toolchain artifact
and its packaging remain the downstream outcome.
