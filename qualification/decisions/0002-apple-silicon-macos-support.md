<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# ADR 0002: Apple Silicon macOS support

Status: accepted for future downstream development and releases
Date: 2026-09-07

## Decision

The downstream program supports Apple Silicon (`aarch64-apple-darwin`) as its
macOS host. Intel macOS is outside the future downstream support and
qualification scope. This does not change upstream Kani's support policy.

Linux coverage remains Ubuntu 22.04 x86_64, Ubuntu 24.04 x86_64, and Ubuntu
24.04 ARM64. Qualification must cover every platform actually claimed by a new
release; reducing macOS scope does not authorize reducing Linux coverage.

The owner explicitly approved removing only `regression (macos-15-intel)` from
the `main` ruleset. The remaining nine required check types are:

- `Qualification tests`
- `audit`
- `clippy-check`
- `format-check`
- `llbc-regression`
- `regression (macos-14)`
- `regression (ubuntu-22.04)`
- `regression (ubuntu-24.04)`
- `regression (ubuntu-24.04-arm)`

At acceptance, ruleset `21527247` was read back as active, with these checks
bound to GitHub Actions integration `15368`, strict up-to-date checking, and
unchanged PR, signature, history, and no-bypass protections. The live ruleset
remains authoritative; check names and runner labels must be revalidated when
implementing workflow changes.

## Rationale and consequences

The program's intended macOS users run Apple Silicon. Maintaining a required
Intel host adds a support obligation outside that intended scope. This is a
declared support decision, not evidence that Intel verification is unsound or
that a slow check can generally be waived.

[#5](https://github.com/rbeauchamp/kani/issues/5) owns removal of Intel jobs from
automatic regression, build, and installation matrices; that implementation is
still pending. [#1](https://github.com/rbeauchamp/kani/issues/1) and
[#8](https://github.com/rbeauchamp/kani/issues/8) apply this scope to future
qualification and packaging. Existing published tags, assets, manifests,
receipts, and release claims remain unchanged.

The 15-minute CI turnaround remains a measurement target. Preserve soundness
coverage before moving suites off PRs, and keep generally useful improvements
separate from fork-specific platform choices under [ADR 0001](0001-upstream-first-qualified-distribution.md).
