<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Candidate assessment: 2026-08 candidate 1

Status: **not promotable yet**

Assessment date: 2026-08-25

Profile: [`core-v1` draft](../profiles/core-v1.md)

## Identity

| Field | Value |
|---|---|
| Upstream repository | `https://github.com/model-checking/kani` |
| Fork repository | `https://github.com/rbeauchamp/kani` |
| Upstream/fork baseline | `4f7baae414d596eaa82ee90ee529f9957ca565dd` |
| Baseline tree | `eff9f33ba2a79a3c815ba58080ccd3395955a88d` |
| Released comparison | `kani-0.67.0` at `4feaaad1d6a2378a6ff6caa3b4fc5d6999c7bb5d` |
| Downstream patches | None at assessment time |
| Rust | `nightly-2026-04-01` |
| CBMC | `6.10.0` |
| Kissat | `4.0.1` |
| Charon | `b250680abd40ff1aaa07081d0497dc2755ed112e` |
| Cargo lock | SHA-256 `086f3cb1e49b5f54f8d2684d89f6fa17c32226202f77fe3c2f5f46205c28589f` |

## Delta from `kani-0.67.0`

- 162 commits;
- 465 changed files;
- 18,550 insertions and 1,972 deletions;
- Rust moved from `nightly-2025-11-21`;
- CBMC moved from 6.8.0; and
- Kissat remained at 4.0.1.

The delta includes meaningful correctness fixes and additional tests. Its size
also means release recency alone cannot establish stability.

## Exact upstream workflow evidence

Successful workflows observed for the candidate SHA:

- [Kani CI 32815085834](https://github.com/model-checking/kani/actions/runs/32815085834);
- [Release Bundle 32815085848](https://github.com/model-checking/kani/actions/runs/32815085848);
- [Kani Format Check 32815085769](https://github.com/model-checking/kani/actions/runs/32815085769);
- [Cargo Deny 32815085896](https://github.com/model-checking/kani/actions/runs/32815085896);
- [end-to-end performance 32815085845](https://github.com/model-checking/kani/actions/runs/32815085845);
- [CodeQL push analysis 32815085184](https://github.com/model-checking/kani/actions/runs/32815085184); and
- [CBMC-latest nightly 32830535495](https://github.com/model-checking/kani/actions/runs/32830535495).

[Compiler-performance run 32815085770](https://github.com/model-checking/kani/actions/runs/32815085770)
built and timed both revisions successfully, then failed the comparison analysis.
The recorded IQR averages identify a potential `s2n-codec` compile-time
regression from about 11.77 seconds to 13.97 seconds. This is P2 performance
evidence rather than evidence of an incorrect verification verdict; the public
corpus must still meet its timeout budgets.

This is upstream integration evidence. It is not yet a downstream artifact or
profile receipt.

## Initial blocker and exclusion ledger

| Item | Candidate disposition | Promotion effect |
|---|---|---|
| Quantifier dropping, PR #4719 | Open, approved, not in baseline | Quantifiers excluded unless the fix is integrated and requalified |
| `Rc`/`Arc` generation, issue #4752 | Open soundness issue | Autoharness and affected nondeterminism excluded |
| `--quiet` exit behavior, issue #4745 | Open | Flag prohibited; gate rejects it |
| JSON partial/clean-pass behavior, issue #4731 | Open | JSON cannot be a verdict oracle |
| Runtime CBMC mismatch, PR #4723 | Open | Independent exact runtime version check required |
| Compiler-performance analysis | Potential `s2n-codec` compile-time regression | Check timeout budgets; does not independently block soundness |
| Open soundness-labeled work | [Profile triage recorded](../evidence/2026-08-25-open-soundness-triage.md) | Mechanical exclusions and explicit residual TCB acceptance required |
| Result-integrity search | [Initial targeted triage recorded](../evidence/2026-08-25-result-integrity-triage.md) | Existing controls recorded; full search and remaining mutations still block promotion |
| Public profile corpus | Not yet frozen and replayed on both platforms | Blocks promotion |
| Negative mutation corpus | False assertion, prohibited quiet result, zero-match filters, and gate parser/policy probes pass; full corpus incomplete | Blocks promotion |
| Artifact SBOM/hash/signature | Hashes captured for initial artifacts; SBOM and signatures not generated | Blocks promotion |

## Current verdict on the release assumption

**Useful new work exists:** supported with high confidence by the exact commit
delta, concrete correctness fixes, updated toolchain, and successful upstream
CI and bundle runs.

**Current `main` is uniformly better than 0.67.0:** not supported. New features
and result paths introduce or expose hazards that must be excluded or fixed.

**A stable qualified release can be produced without fixing the full backlog:**
supported for continued qualification, but conditional on the complete public
profile gate, Linux replay, mutation success, independent review, and final
artifact qualification.

The dated bootstrap execution record is
[`2026-08-25-macos-arm64-candidate-1.md`](../evidence/2026-08-25-macos-arm64-candidate-1.md).

## Required evidence before promotion

- resolve every pending platform and runner identity;
- freeze and replay the complete public profile corpus;
- complete the known-hazard dispositions;
- complete unlabeled P0/P1 discovery beyond the soundness-label triage;
- run `core-v1` on every claimed platform from the built artifacts;
- obtain independent exact-head semantic review;
- store a complete immutable receipt; and
- verify signed publication metadata and artifacts after release.
