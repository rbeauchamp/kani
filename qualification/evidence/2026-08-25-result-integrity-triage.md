<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Result-integrity issue triage for `core-v1`

Status: **initial targeted search; not a complete issue audit**

Snapshot date: 2026-08-25

## Scope and method

This pass searched open public Kani issues for successful-verdict text, exit
codes, silent behavior, missing or partial results, cover reachability, and
multi-harness execution. It then read each selected report rather than inferring
impact from its title or label.

The purpose is to harden `core-v1` against false or ambiguous success. The list
is not exhaustive; a full open-issue/body search and source-path audit remain
promotion gates.

## Disposition table

| Item | Result-integrity risk | `core-v1` control |
|---|---|---|
| [Issue #4745](https://github.com/model-checking/kani/issues/4745) | A failing `--quiet` run can exit zero with no output | Prohibit `--quiet` and `-q` before execution; red probe must demonstrate rejection |
| [Issue #4731](https://github.com/model-checking/kani/issues/4731) | Failed, empty, or partial JSON export can appear clean | Do not use `--export-json` as a verdict oracle; require independent terminal parsing |
| [Issue #4729](https://github.com/model-checking/kani/issues/4729) | `--fail-fast` drops completed harness results and misstates totals | Prohibit `--fail-fast`; require exact selected, checked, and summarized harness equality |
| [Issue #4438](https://github.com/model-checking/kani/issues/4438) | Multi-threaded output can interleave harness starts and results | Prohibit `--jobs`, `-j`, and autoharness; execute the public gate sequentially |
| [Issue #2792](https://github.com/model-checking/kani/issues/2792) | An unsatisfied cover property does not make the harness fail | Independently require every declared cover property to be satisfied |
| [Issue #4590](https://github.com/model-checking/kani/issues/4590) | A Z3-backed harness diverges across x86_64 and ARM and can surface a backend error | Qualify only the pinned default CBMC/Kissat path initially; require platform agreement and fail on backend errors |
| [Issue #3240](https://github.com/model-checking/kani/issues/3240) | An unreachable `unreachable!()` property can be displayed as `SUCCESS` rather than `UNREACHABLE` | Do not infer reachability solely from the status label; inventory source obligations, covers, assumptions, and exact property results |
| [Issue #1711](https://github.com/model-checking/kani/issues/1711) | A large result block may not visibly identify its harness | Bind each checked-harness marker to the exact requested set and require one unique terminal summary |
| [Issue #2648](https://github.com/model-checking/kani/issues/2648) | Concurrent Kani processes can race on shared build artifacts | Use a unique clean checkout and target directory per gate; prohibit simultaneous gates sharing build state |
| [Issue #286](https://github.com/model-checking/kani/issues/286) | Expected-failure regression conventions can silently stop testing the intended path | Use explicit red mutations whose command, nonzero status, and expected failure marker are independently checked |

## Gate implications

The generic public gate already rejects quiet, JSON export, fail-fast, parallel
jobs, autoharness, and non-terse output. It requires exact harness inventory,
one checked marker per harness, one all-green terminal summary, every successful
verification marker, all covers, the expected unreachable distribution, and an
exact diagnostic ledger.

Remaining work includes dedicated mutations for shared-build-state races,
truncated output, backend error, runtime-version substitution, inadequate
unwind, and source-level vacuity. Platform agreement must be measured on the
exact built artifacts; it cannot be inferred from one architecture.
