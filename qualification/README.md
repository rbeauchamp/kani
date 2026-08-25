<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Qualified Kani downstream program

This directory is the source of truth for an unofficial downstream qualification
program built on Kani. Its purpose is to make carefully selected upstream progress
available to mission-critical downstream users sooner while preserving explicit
scope, reproducible evidence, and a small upstreamable patch delta.

Kani is a substantial verifier created and maintained by the
[Kani project](https://github.com/model-checking/kani). This program is grateful
for that work. It is intended to complement upstream development, contribute
generally useful improvements back, and avoid creating unnecessary semantic
divergence. It is not sponsored or endorsed by the upstream Kani team.

"Qualified" has a deliberately narrow meaning: a named release candidate passed
the recorded gate for a declared profile, toolchain, platform set, and proof
corpus. It does **not** mean that Kani, CBMC, rustc, or the full Rust language has
been proved correct.

## Authority and navigation

When records disagree, use this order:

1. [`roadmap.md`](roadmap.md) owns work ordering, live status, promotion gates,
   and the next action.
2. [`operating-model.md`](operating-model.md) owns the normative development,
   review, qualification, and release process.
3. A file under [`profiles/`](profiles/) owns the exact supported envelope for a
   release.
4. [`manifests/`](manifests/) contains machine-checked toolchain and consumer
   identities, harness sets, result expectations, and diagnostic ledgers.
5. [`scripts/`](scripts/) implements fail-closed gates for those manifests.
6. A file under [`releases/`](releases/) owns immutable candidate identity,
   evidence, exceptions, and the terminal release verdict.
7. [`evidence/`](evidence/) contains dated, exact-candidate execution records;
   release files select which records belong to the terminal receipt.
8. [`research-basis.md`](research-basis.md) records the evidence and reasoning
   behind this operating model.
9. [`decisions/`](decisions/) records durable architectural decisions and their
   supersession history.

Git objects, GitHub check readback, signed release metadata, and stored
qualification receipts remain the authority for exact identities and observed
results. Narrative documents must link to those objects rather than replacing
them.

Automation is an evidence generator, not a promotion authority. A passing gate
whose manifest still has `bootstrap` status cannot support a stable release.

## Current state

The fork was created on 2026-08-25. The first candidate assessment is recorded
in [`releases/2026-08-candidate-1.md`](releases/2026-08-candidate-1.md). No
qualified release has been promoted yet.

## Public collaboration standard

Public issues, pull requests, comments, and release notes must:

- be respectful, factual, and technically scoped;
- acknowledge upstream work and avoid attributing motive or fault;
- distinguish downstream requirements from upstream obligations;
- keep each contribution focused and easy to review;
- use Kani's private security channel for sensitive security findings; and
- avoid implying upstream endorsement of this downstream distribution.
