<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Program navigation

[GitHub issues](https://github.com/rbeauchamp/kani/issues) are the live backlog;
the [Kani macOS Verification Engine project](https://github.com/users/rbeauchamp/projects/4)
owns work ordering and status. Each issue contains its acceptance criteria,
dependencies, exact baseline, and completion evidence. This file provides stable
navigation, without duplicating task checklists or current completion claims.

| Order | Workstream | Tracking |
|---|---|---|
| 0 | Complete qualification infrastructure and enforce essential CI | [#11](https://github.com/rbeauchamp/kani/issues/11), [PR #10](https://github.com/rbeauchamp/kani/pull/10) |
| 1 | Synchronize upstream with preserved ancestry | [#4](https://github.com/rbeauchamp/kani/issues/4) |
| 2 | Remove duplicate CI execution and unnecessary benchmark triggers | [#5](https://github.com/rbeauchamp/kani/issues/5) |
| 3 | Align compiler semantics and tool identities with Rust 1.97.1 | [#1](https://github.com/rbeauchamp/kani/issues/1) |
| 4 | Close applicable soundness obligations | [#7](https://github.com/rbeauchamp/kani/issues/7) |
| 5 | Improve performance where measurements justify changes | [#6](https://github.com/rbeauchamp/kani/issues/6) |
| 6 | Package and distribute a qualified artifact | [#8](https://github.com/rbeauchamp/kani/issues/8) |

The umbrella roadmap is [#9](https://github.com/rbeauchamp/kani/issues/9).
Infrastructure completion does not close compiler alignment or qualify a release.
The proposed CI turnaround is a measurement target until observed at the relevant
commit and runner set.

## Promotion contracts

The [operating model](operating-model.md) owns development, review, qualification,
and release requirements. A release must freeze the supported
[profile](profiles/), public corpus, assumptions, diagnostics, source identities,
and artifact identities; resolve or mechanically exclude intersecting soundness
hazards; pass all applicable gates and red mutations; and retain independent
review and immutable receipts. Full qualification retains its declared platform
coverage even when routine PR checks are optimized.

A bootstrap receipt or an infrastructure self-test cannot authorize stable
promotion. Published artifacts are never changed to match later development.
Corrections require a new, explicitly superseding release record. Exact Git
objects, checks, artifact hashes, and stored receipts take precedence over prose.

The [published controlled-use prerelease](releases/qualified-0.67.0+20260825.1/release-scope.md)
and its [publication readback](releases/qualified-0.67.0+20260825.1/publication-readback.md)
remain historical evidence for their exact identities and narrow envelope.
Application acceptance belongs in each adopting application's own system and
cannot broaden the public claim.

Generally useful fixes should remain small and upstreamable. A permanent semantic
fork requires a superseding [decision](decisions/) under the operating model's
ownership and sustainability criteria.
