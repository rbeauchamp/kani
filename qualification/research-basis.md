<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Research basis for an upstream-first qualified distribution

## Research question

How can downstream consumers obtain current Kani capabilities for
mission-critical verification, contribute useful fixes upstream, and produce a
stable release promptly without overstating what has been assured?

This is a dated decision record, not a permanent description of upstream. Facts
below were captured on 2026-08-25 and must be refreshed before later promotion
decisions.

## Executive finding

The evidence supports an upstream-first qualified distribution rather than an
immediate independent semantic fork. Upstream development is active, while the
latest public release does not contain 162 subsequent commits. A downstream
distribution can bridge that difference in timing by pinning an exact upstream commit,
carrying only reviewed temporary patches, and publishing a restricted
qualification envelope.

The evidence does **not** support tagging current `main` as generally stable
without further work. Current `main` contains useful correctness and capability
improvements, but several open issues and pull requests can affect false-success
behavior or result integrity. The first release therefore needs explicit
exclusions, fail-closed wrappers, downstream proof-corpus replay, and negative
mutations.

## Method

The initial research used five angles: project activity and release health,
backlog composition, open correctness work, verifier architecture and trusted
computing base, and release/qualification engineering. It inspected more than
30 primary GitHub pages, repository files, documentation pages, issue records,
pull requests, and workflow results. Three independent research lanes examined
project health, assurance boundaries, and fork strategy. Four central claims
received adversarial review: three survived with qualifications; one claim about
the causal role of AI throughput was narrowed because the public evidence proved
AI participation, not its precise share of implementation or the project's
limiting factor.

The bootstrap pass then reproduced the central counts and exact candidate facts
from the GitHub API and the cloned repository. No secondary reporting is needed
for the decisions below.

## Evidence snapshot

| Observation | Evidence captured 2026-08-25 | Relevance |
|---|---|---|
| Latest release | [`kani-0.67.0`](https://github.com/model-checking/kani/releases/tag/kani-0.67.0), published 2026-01-16 | The consumable release is materially behind development. |
| Candidate baseline | Upstream and fork `main` at `4f7baae414d596eaa82ee90ee529f9957ca565dd` | All qualification claims can bind to one exact source identity. |
| Delta from release | 162 commits; 465 files changed; 18,550 insertions and 1,972 deletions | The delta is too large to infer safety from release age or commit count. |
| Recent upstream activity | 62 `main` commits and 62 merged pull requests since 2026-07-26 | Upstream maintenance is active; continuous intake has material value. |
| Open work | 455 issues and 25 pull requests; 148 issues labeled `[C] Bug`, 29 `[F] Crash`, and 11 `[F] Soundness` | Raw counts describe discovery, not a release-priority queue; labels overlap. |
| Soundness milestone | [16 open and 11 closed](https://github.com/model-checking/kani/milestone/8), with no due date | Known soundness work remains; milestone membership alone cannot define our profile. |
| Toolchain movement | Rust `nightly-2025-11-21` to `nightly-2026-04-01`; CBMC 6.8.0 to 6.10.0; Kissat remains 4.0.1 | The distribution must bind the entire toolchain tuple, not only a Kani SHA. |
| Candidate CI | Kani CI, format, dependency audit, CodeQL, release-bundle, end-to-end performance, and CBMC-latest workflows succeeded; the long compiler-performance analysis failed | Upstream CI is strong evidence but is not a downstream qualification certificate. The performance failure needs disposition. |

Counts can move between API calls. Promotion receipts must store their own
capture time and queries rather than copying this table indefinitely.

## Evidence that current `main` contains useful work

The post-release delta includes concrete correctness and result-integrity work,
including:

- `fb037f320`: model missing undefined behavior when offsets wrap CBMC's pointer
  encoding;
- `61ac6c84f`: check that fast-math intrinsic results are finite;
- `46aa1a914`: fail a zero-match harness filter before code generation and
  export;
- `e3332264a`: fix a compiler crash for non-literal assertion and cover
  messages; and
- newer pinned Rust and CBMC versions, broader regression coverage, release
  bundles, and platform testing.

These changes support the proposition that a newer downstream release can be
more useful than 0.67.0 for a declared profile. They do not prove that every
post-release change is an improvement for every profile.

## Known current hazards relevant to promotion

The following items are examples of why promotion must be profile-specific:

- [PR #4719](https://github.com/model-checking/kani/pull/4719), approved and open
  at `32af783b15617a996685b813c30dbe863f26b7e9`, makes verification fail when a
  solver backend drops symbolic-bound quantifiers. Until resolved and
  requalified, experimental quantifiers are outside the first profile.
- [Issue #4752](https://github.com/model-checking/kani/issues/4752) reports that
  nondeterministic `Rc<T>` and `Arc<T>` generation fixes `strong_count` at one,
  which can support a false property. Autoharness and nondeterministic smart
  pointers are outside the first profile.
- [Issue #4745](https://github.com/model-checking/kani/issues/4745) reports that
  `--quiet` can make a failing run exit successfully. The first profile forbids
  `--quiet` and independently validates terminal output and expected harnesses.
- [Issue #4731](https://github.com/model-checking/kani/issues/4731) reports that
  failed, empty, or partial `--export-json` runs can serialize as a clean pass.
  JSON export is not a release oracle until this is resolved and requalified.
- [PR #4723](https://github.com/model-checking/kani/pull/4723) addresses mismatch
  between the runtime CBMC and Kani's pinned version. The distribution must
  check the runtime tool version independently.

These are not criticisms of upstream engineering. They are public examples of
the failure modes that a mission-critical downstream qualification process must
model explicitly.

## Reasoning ledger

### Active development plus a lagged public release

- Evidence: 62 recent commits and merged pull requests, but 162 commits since
  the last release.
- Inference: continuing to ingest upstream work is valuable, while waiting only
  for public releases does not meet the downstream currency requirement.
- Decision: keep fork `main` as a mirror and qualify exact upstream snapshots on
  separate branches.
- Falsifier: repeated rejection or prolonged non-review of required P0/P1 fixes,
  or deliberate semantic divergence, can justify a long-lived fork later.

### A raw backlog is not a qualification plan

- Evidence: issue categories overlap, much of the backlog is old or unassigned,
  and current false-success hazards are not consistently labeled.
- Inference: sorting all 455 issues by age or label can miss mission-critical
  risks while consuming effort on irrelevant features.
- Decision: triage against named downstream profiles, prioritizing false
  success and result integrity before crashes, false failures, performance, or
  convenience.
- Falsifier: a comprehensive audit may later show that a broader class is needed
  for a chosen profile; the profile and roadmap then expand explicitly.

### Green upstream checks are necessary but not sufficient

- Evidence: the exact candidate has broad successful CI and bundle tests while
  known false-success issues remain outside or ahead of those suites.
- Inference: CI proves only the encoded tests and workflow conditions.
- Decision: require downstream corpus replay, expected harness manifests,
  reachability obligations, and negative mutations at the exact candidate head.
- Falsifier: none. This is an assurance boundary, not a temporary project-state
  claim.

### “Verified correct” would overstate the evidence

- Evidence: Kani translates Rust MIR into CBMC's GOTO representation and relies
  on rustc, Kani models, CBMC, solvers, platform assumptions, and harness
  assumptions. Kani documents bounded-verification and feature limitations, and
  [issue #310](https://github.com/model-checking/kani/issues/310) tracks backend
  correctness auditing.
- Inference: fixing known bugs cannot prove this complete trusted computing base.
- Decision: release only as “qualified for profile X at exact identity Y,” with
  an explicit TCB and exclusions.
- Falsifier: a future end-to-end refinement and trusted-tool proof could narrow
  the TCB, but it would still need an exact configuration binding.

### AI capacity changes throughput, not the oracle problem

- Evidence: upstream publishes AI-assistant guidance and has visible AI-assisted
  implementation and review, but public metadata does not establish the precise
  fraction or causal bottleneck.
- Inference: additional AI capacity can accelerate triage, reduction, testing,
  review, and maintenance; it cannot make an author's self-review independent.
- Decision: separate authoring, adversarial review, and promotion roles and bind
  conclusions to executable evidence.

## Alternatives considered

### Wait for the next upstream release

This preserves the smallest maintenance burden but does not satisfy the current
need for a recent toolchain or give downstream users control over qualification
timing. It remains useful as a comparison baseline.

### Tag upstream `main` immediately

This is fast but would convert successful upstream CI into an unsupported claim
of downstream stability. It was rejected because the consumer corpus, known
hazard exclusions, runtime dependency checks, and mutation gate are not yet
complete.

### Close or review the entire backlog before releasing

This is neither necessary nor risk-directed. The first release can be stable for
a restricted profile while explicitly excluding unresolved features.

### Begin with a permanent semantic fork

This would immediately inherit Rust-nightly, compiler, Charon, CBMC, solver,
platform, and release maintenance while forfeiting rapid upstream integration.
It remains an escape hatch if measured upstream collaboration cannot satisfy
required semantics or response windows.

## Caveats and open questions

- The exact public profile corpus must be frozen before promotion.
- The long compiler-performance workflow failure at the candidate SHA needs a
  documented root cause and release-impact verdict.
- Platform qualification cannot be inferred from another architecture.
- Security-sensitive discoveries must use Kani's private disclosure process,
  not this public evidence ledger.
- All activity and backlog measurements are time-sensitive.

## Primary sources

- [Kani releases](https://github.com/model-checking/kani/releases)
- [Kani commit history](https://github.com/model-checking/kani/commits/main)
- [Kani open issues](https://github.com/model-checking/kani/issues?q=is%3Aissue%20state%3Aopen)
- [Kani open pull requests](https://github.com/model-checking/kani/pulls?q=is%3Apr%20state%3Aopen)
- [Kani Soundness milestone](https://github.com/model-checking/kani/milestone/8)
- [`AGENTS.md`](../AGENTS.md), including Kani's soundness-first principle
- [`CONTRIBUTING.md`](../CONTRIBUTING.md)
- [release workflow](../.github/workflows/release.yml)
- [soundness documentation](../docs/src/soundness.md)
- [verification-result documentation](../docs/src/verification-results.md)
- [regression-testing documentation](../docs/src/regression-testing.md)
- [Kani 2026 architecture paper](https://arxiv.org/abs/2607.01504)
