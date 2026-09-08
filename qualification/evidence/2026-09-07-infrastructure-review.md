<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Qualification infrastructure closeout and synchronization basis

Date: 2026-09-07
Scope: infrastructure acceptance and preparation for upstream synchronization;
this record is not qualification of a new release.

## Exact source and merge evidence

[PR #10](https://github.com/rbeauchamp/kani/pull/10) was reviewed at
`8678f88e5fa681b6bc339e269505a1b90b83172b` and squash-merged as
`934273e6d909be63df84a588a6514c7afaca0625`. GitHub reported `MERGED` at
2026-09-07 23:11:05 UTC. The merged tree was compared with the reviewed head
using `git diff --exit-code`; they were identical. The merge was verified as an
ancestor of `origin/main`, and the primary checkout was fast-forwarded to it.

[#11](https://github.com/rbeauchamp/kani/issues/11) closed automatically; its
project entry and PR #10's entry were read back as Done. The local feature
branch was removed after ancestry verification. GitHub automatically deleted
the remote feature branch, and its stale tracking ref was removed.

## Acceptance and its limits

Focused independent review covered runtime identity, complete result admission,
partial execution rejection, and immutable receipt publication. Targeted repair
verification closed the accepted findings. A final reuse, quality, and efficiency
pass removed redundant ownership and temporary-directory management.

The executable contracts and residual trusted boundaries are documented in
[`infrastructure.md`](../infrastructure.md). In particular:

- Infrastructure mutation self-tests do not establish qualification against a
  pinned release manifest. The pinned-manifest path rejects identity mismatches
  before proof execution.
- Composition validates complete producer records; it cannot authenticate an
  external producer or prove archive-to-executable provenance.
- The count proof includes the production guard directly and covers all four
  `u32` inputs. It does not prove the parser, OS runner, compiler, or solver.
- The runner requires normal termination and EOF on both streams. Its supported
  Unix tools wait for their children and do not daemonize; it is not an
  arbitrary-process sandbox.
- Receipt publication does not overwrite existing destinations. Subsequent
  filesystem access and release custody remain separate trusted boundaries.

These boundaries replace rejected shortcuts: invented process metadata from
raw logs, a fallback label hash when the executable cannot be hashed, accepting
passing prefixes without EOF, interpreting crashes as successful negative
probes, and using self-test receipts to imply pinned qualification.

## Verification evidence

The [qualification CI run](https://github.com/rbeauchamp/kani/actions/runs/34165747297)
rebuilt Kani at the reviewed PR head and passed 28 Rust checks, 9 existing Python
policy checks, the source-linked count proof, and all seven mutation probes.
Local compiler/check/Clippy/formatting checks also passed. The local count proof
reported 12 successful checks. These observations are not universal verifier
soundness evidence.

A local pinned-manifest attempt rejected CBMC 6.8.0 versus the manifest's 6.10.0
before executing probes and published no receipt. Local exploratory execution
used an existing development compiler; it was not claimed as a rebuild of the
reviewed compiler. CI supplied the exact-source rebuild evidence.

The [PR regression run](https://github.com/rbeauchamp/kani/actions/runs/34165747319),
[format/Clippy run](https://github.com/rbeauchamp/kani/actions/runs/34165747348),
and [audit run](https://github.com/rbeauchamp/kani/actions/runs/34165747304)
provided the other required checks. All nine required check types under
[ADR 0002](../decisions/0002-apple-silicon-macos-support.md) passed before merge.
Intel macOS and an optional end-to-end benchmark were still running and were
not asserted to have passed. The
[bundle workflow](https://github.com/rbeauchamp/kani/actions/runs/34165747327)
also passed its builds and installation checks; publication jobs were skipped.

Reported PR regression job durations were 28m27s for Ubuntu 22.04, 26m54s for
Ubuntu 24.04, 27m8s for Ubuntu 24.04 ARM64, and 30m44s for Apple Silicon macOS.
They ran in parallel. These are individual observations, not a latency guarantee
or a demonstration of the 15-minute target. Duplicate push/PR jobs and automatic
benchmarks still need the workflow changes tracked in #5.

The macOS dependency-setup failure found during review was repaired by removing
the standalone Homebrew tap and installing the fully qualified pinned CBMC
formula directly. New-head macOS setup and bundle checks passed. Do not retry
the obsolete failed head or reinstate the standalone tap without new evidence.

Published prerelease `qualified-0.67.0+20260825.1` identity and asset metadata were
compared before and after merge and were unchanged. Existing manifest, release,
and evidence files were unchanged by PR #10.

## Synchronization preview, not an executed merge

The refreshed preparation used:

| Object | Identity |
|---|---|
| Fork main basis | `934273e6d909be63df84a588a6514c7afaca0625` |
| Prior upstream synchronization | `5da96e5aac750664c8d383f66e8b561d7718d81a` |
| Selected upstream commit | `b07abe8a72f8eb1ef1ea3521ca6b00a973d341bc` |
| Selected upstream tree | `f680110e42c74771f1696e7a60481e65343754e0` |
| Preview merge tree | `76cfc7390797c135430938579fd242159b47b4c1` |

After fetching both remotes, `git merge-tree --write-tree origin/main
upstream/main` succeeded without conflicts. The target contains 17 commits after
the prior synchronization. The preview's compiler, driver, bindings, and library
trees matched the selected upstream exactly. Existing qualification manifests,
release files, and evidence were unchanged. This preview did not build or verify
the synchronized compiler and did not create a merge commit or change a branch.
Recompute it when either parent changes; [#4](https://github.com/rbeauchamp/kani/issues/4)
owns the executable plan and acceptance criteria.

Upstream's CBMC version warning is useful diagnostic behavior; a warning alone
does not enforce this program's fail-closed runtime identity contract. The
historical 62 compiler errors describe an older base and are not an assessment
of exact Rust 1.97.1 compatibility after synchronization.
