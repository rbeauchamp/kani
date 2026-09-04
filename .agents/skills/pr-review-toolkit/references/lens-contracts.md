# Lens contracts (Kani)

## Contents

- Shared reviewer contract
- Targeted repair verification contract
- General code lens
- Comment lens
- Guarantee-coverage lens
- Silent-failure lens
- Type-design lens
- Precedent and blast-radius lens
- Required simplification reuse lens
- Required simplification quality lens
- Required simplification efficiency lens

## Shared reviewer contract

Prefix every lens prompt with this contract:

```text
Review only the frozen change set described below. Work independently and do not edit files, run formatters that write, stage changes, commit, push, post comments, or mutate external systems.

Read the applicable AGENTS.md, CONTRIBUTING.md, and nested repository instructions. Inspect the actual diff and live source; do not rely only on the patch when call sites, proofs, audits, or contracts are needed. Use rg first for code search. Ignore pre-existing unrelated problems and cosmetic preferences unsupported by project rules.

Complete the entire lens before responding: inspect every changed hunk plus its relevant callers, unit tests, qualification probes, and contracts; do not stop after finding one or several issues. Then make a second internal pass over the same frozen snapshot. These are coverage passes inside one discovery invocation, not the definition of done or a review-round budget. For each new enforcement boundary or claimed invariant, attempt a discriminating counterexample or bypass mutation in a disposable copy when practical. Verify external-tool semantics against the exact pinned version's help or primary source.

Return high-confidence, actionable findings only. For each finding provide:
- stable lens-prefixed ID
- severity (`P0`, `P1`, `P2`, or `P3`) and confidence from 0-100
- absolute or repository-relative file path and tight line reference
- concrete evidence from the changed code and relevant context
- provenance: ORIGINAL_DIFF, FIX_REGRESSION, DEPENDENT, or PREEXISTING
- the acceptance criterion, repository rule, or invariant at risk
- user or system impact
- smallest correct fix
- verification that would prove the fix

End with a compact coverage attestation naming reviewed paths/hunks, callers/contracts inspected, probes attempted, and any unreviewed surface. CLEAN is valid only when no relevant surface remains unreviewed. If the lens does not apply, return NOT_APPLICABLE and the exact reason. Do not include praise or speculative suggestions. Do not expand beyond the supplied review charter; identify evidence that would require a wider architecture change as SCOPE_EXPANSION for the parent to decide.
```

## Targeted repair verification contract

Use this contract only after the parent has accepted and repaired one or more findings that share an invariant or risk domain:

```text
Verify only the supplied finding IDs, repaired invariant, exact repair diff, and impacted callers, tests, and qualification probes. Work independently and do not edit files, run formatters that write, stage changes, commit, push, post comments, or mutate external systems.

Read the applicable repository instructions and live source. Reproduce the original counterexample or inspect its evidence, then attempt a discriminating bypass or mutation against the repair. Check the supplied deterministic proof and exact tool semantics where relevant. Do not restart whole-diff discovery and do not report unrelated observations as new review findings.

For every supplied finding ID, return exactly one status:
- RESOLVED: the repair closes the named invariant with adequate evidence;
- UNRESOLVED: the original failure or a causally linked repair regression/dependent defect remains;
- SCOPE_INVALIDATED: the actual repair impact exceeds the charter or invalidates prior discovery coverage.

For UNRESOLVED, give the concrete counterexample, tight repair location, smallest correction, and proof required. You may add a new stable ID only for a causally linked repair regression or dependent defect. Mention unrelated observations separately for the parent to ledger, but do not explore them or expand scope.

End with a compact attestation naming the repair hunks, impacted contracts, probes, and deterministic evidence checked for each finding ID. Do not include praise or speculative suggestions.
```

## General code lens

```text
Lens: general code correctness and compiler/verifier soundness.

Review for actual logic bugs, race conditions, invalid assumptions, unhandled error conditions, unsound verifier logic, security or privacy regressions, resource leaks, broken API contracts, performance regressions, and explicit repository-instruction violations. Check whether the implementation follows established Kani and Rust conventions without demanding unrelated refactors.

Core Principle: "Soundness over convenience: Kani must never produce false negatives (saying code is safe when it isn't)."

Report only findings with confidence at least 80. Use P0 for an immediate release-stopping safety, soundness, or data-loss defect; P1 for a concrete high-impact correctness, acceptance, or repository-rule failure; P2 for a concrete medium-impact defect; and P3 for a low-impact but still actionable defect.
```

## Comment lens

```text
Lens: comment and documentation truth.

Cross-check changed comments, docstrings, README text, and explanatory comments against code behavior. Find factually wrong or misleading claims, stale references, missing non-obvious invariants or side effects, temporary reasoning presented as permanent truth, and comments that merely narrate obvious code and create maintenance debt.

Check license headers: all new files must have the dual Apache-2.0 / MIT header:
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT (or # for shell/yaml scripts)

Recommend removal only when a comment has no durable why, invariant, external constraint, or operational value. Report concrete rewrites or removals, not general documentation preferences.
```

## Guarantee-coverage lens

```text
Lens: guarantee coverage — who owns each property.

Map changed behavior, CLI flags, and failure modes to the artifact that owns each guarantee:
- A fail-closed red-mutation safety probe (in `qualification/fixtures/mutations/` and `tools/kani-qualify/src/mutations.rs`)
- Unit tests (`cargo test -p kani-qualify`)
- Verifier end-to-end harness tests (`tests/kani/`)
- Diagnostic & output oracle tests (`tests/expected/`)
- Cargo integration tests (`tests/cargo-kani/`)

Find guarantees nothing owns, qualification probes that miss known bypasses, parsers that allow unhandled status codes to pass as success, and tests that test trivial invariants rather than edge/failure cases.

For every gap, name the exact unit test, red-mutation probe, or compiletest case that would close it. Soundness is non-negotiable: verifier and qualification tooling must fail-closed.
```

## Silent-failure lens

```text
Lens: silent failures, error handling, and process exit codes.

Trace changed error, fallback, nullable, retry, async, logging, process execution, cancellation, and cleanup paths. Find swallowed or over-broad exceptions, default/null returns that hide failure, unjustified fallback behavior, unhandled child process exit codes (e.g., CBMC or kani crashing or returning non-zero), regex parsing that ignores unexpected status lines, fire-and-forget work, and cleanup that masks original errors.

Calibrate to Kani's fail-closed verification qualification standards:
- Any unparsed verification output MUST NOT be treated as a pass.
- Child process timeouts, termination signals, and non-zero exit codes must be explicitly surfaced as failures.
- CLI exit codes must reflect verification status (non-zero on qualification failure).
```

## Type-design lens

```text
Lens: type design, models, and invariants.

Inspect changed Rust types, structs, enums, serialization models (`serde::Serialize`, `Deserialize`), CLI arguments (`clap`), and receipts. Identify domain invariants and determine whether invalid states can be constructed or introduced through deserialization, default values, stringly-typed fields, or missing validation.

Check:
- Exhaustive match expressions (avoid `_` wildcards on domain status enums).
- Domain types and typed enums for verdicts (`Pass`, `Fail`, `Error`, `Timeout`) rather than raw strings or bools.
- Validation at construction time for models and receipts.
- Round-trip serialization fidelity.

Report only pragmatic improvements that prevent concrete bugs or materially clarify a domain contract.
```

## Precedent and blast-radius lens

```text
Lens: precedent and blast radius.

Inspect changes to root `Cargo.toml`, `Cargo.lock`, CI workflows (`.github/workflows/`), and repository tooling conventions. Check:
- Workspace membership: tool additions are properly declared in workspace `Cargo.toml`.
- Dependency choices: dependencies align with existing workspace versions and licenses.
- CI workflows: new workflows or updated workflows are properly gated, do not run unnecessary matrix jobs, and pass CI checks.
- Blast radius: tool changes do not inadvertently alter the behavior of `kani-compiler`, `kani-driver`, or standard verification runs.
```

## Required simplification reuse lens

```text
Lens: required final simplification through code reuse. This review occurs once after behavior stabilizes. Review the supplied frozen stabilized snapshot only and do not edit files.

For each change:
1. Search for existing utilities, helpers, or standard library APIs that could replace newly written code. Use `rg` first.
2. Flag new functions or structs that duplicate existing functionality in the workspace or dependencies (e.g. `anyhow`, `serde`, `tempfile`, `regex`, `std::process`).
3. Flag inline logic that could use an existing utility, such as hand-rolled string handling, path canonicalization, command construction, or ad-hoc JSON handling.
4. Prefer established local APIs and patterns over new custom abstractions.

Preserve exact behavior and project constraints. This lens always applies; return `CLEAN`, not `NOT_APPLICABLE`, when no finding exists. Return only high-confidence findings with `SIM-REUSE-` IDs and the simplest concrete fix. Valid findings are mandatory fixes by the parent agent.
```

## Required simplification quality lens

```text
Lens: required final simplification through code quality. This review occurs once after behavior stabilizes. Review the supplied frozen stabilized snapshot only and do not edit files.

Review for:
1. Redundant state: values cached or stored when they can be derived directly.
2. Parameter sprawl: functions taking too many flags or boolean arguments that indicate a missing enum or struct.
3. Copy-paste with slight variation: near-duplicate blocks (e.g., across mutation probe executions) that can be parameterized cleanly.
4. Leaky abstractions: callers reaching into implementation details or private modules.
5. Stringly typed code: raw strings where existing enums or domain types should be used.
6. Unnecessary nesting: deeply nested `if let`, `match`, or loop blocks that can be flattened with early returns or combinators (`?`, `map`, `and_then`).
7. Unnecessary comments: comments that explain what well-named code already says. Keep comments only for non-obvious why, invariants, external constraints, and fail-closed rationale.

Preserve exact behavior and project constraints. Return only high-confidence findings with `SIM-QUALITY-` IDs and the simplest concrete fix. Valid findings are mandatory fixes by the parent agent.
```

## Required simplification efficiency lens

```text
Lens: required final simplification through efficiency. This review occurs once after behavior stabilizes. Review the supplied frozen stabilized snapshot only and do not edit files.

Review for:
1. Unnecessary work: redundant regex compilation (compiling regexes inside loops rather than with `LazyLock` or `once_cell`), repeated file reads, redundant serializations.
2. Unnecessary allocations: excessive `to_string()`, `clone()`, or vector allocations in hot loops or log parsing.
3. Hot-path I/O: unbuffered file reading/writing for large verification outputs.
4. TOCTOU existence checks: checking file existence before opening rather than handling `NotFound` directly.
5. Child process handling: unbounded command execution or zombie child processes.

Preserve exact behavior and project constraints. Return only high-confidence findings with `SIM-EFFICIENCY-` IDs and the simplest concrete fix. Valid findings are mandatory fixes by the parent agent.
```
