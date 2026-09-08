<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Continue the downstream Kani program

Read [session-context.md](session-context.md), the applicable repository guidance,
and the [live roadmap #9](https://github.com/rbeauchamp/kani/issues/9). Complete
the remaining program through #4 → #5 → #1, then #7 soundness, measurement-justified
#6 performance work, and #8 packaging of a qualified Apple Silicon artifact.
The roadmap and each linked issue own forward decisions and acceptance criteria.

Begin with [#4's Step 0](https://github.com/rbeauchamp/kani/issues/4): capture the
saved handoff commit, verify a clean checkout, fetch origin and upstream, and
reconcile the live fork/upstream identities and gates before creating the
synchronization branch. Preserve the handoff when moving onto that branch.
The first gate is a pinned parent pair and a reviewed merge preview that preserves
upstream core identity and immutable release evidence.

Complete each issue's verified integration and tracking before advancing. Do not
stop after #4 or treat PR #10's infrastructure evidence as release qualification.
