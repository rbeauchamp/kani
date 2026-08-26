<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# ADR 0001: Upstream-first qualified distribution

Status: accepted for bootstrap
Date: 2026-08-25

## Context

Downstream mission-critical work needs a more current Kani distribution than
the latest public release. Upstream development is active and contains valuable
post-release fixes, but a current commit plus green CI is not by itself a
qualification certificate. Beginning with a permanent semantic fork would also
create a large compiler, backend, solver, platform, and release-maintenance
obligation.

## Decision

Maintain:

1. a public fork that tracks upstream and contributes focused changes back; and
2. a controlled downstream distribution made from an exact upstream commit plus
   the smallest necessary reviewed patch queue.

Publish releases only for a declared profile with exact-head regression,
downstream proof-corpus, reachability, negative-mutation, independent-review,
artifact, and publication evidence.

Use “qualified for profile X at exact identity Y,” never “verified-correct Kani.”

## Consequences

Benefits:

- downstream users can obtain current fixes without waiting for a general
  upstream release;
- most compiler and toolchain maintenance remains shared with upstream;
- fixes can be reviewed and merged upstream;
- release claims remain bounded and reproducible; and
- a small patch queue is easier to audit and retire.

Costs:

- qualification and release custody remain substantial work;
- each upstream rebase invalidates candidate receipts;
- supported behavior is narrower than all of Kani; and
- consumers must honor exclusions and exact toolchain pins.

## Relationship to upstream

This decision reflects downstream assurance and timing needs, not a judgment on
the upstream team's effort or priorities. The program is thankful for Kani and
will collaborate respectfully, follow contribution and security policies, and
avoid implying upstream endorsement.

## Reconsideration

Supersede this ADR only if measured evidence satisfies the full-fork triggers in
`operating-model.md` and durable independent maintenance and assurance ownership
have been approved.
