---
name: pr-review-toolkit
description: Runs a comprehensive, evidence-bounded review-and-fix workflow over current changes, a branch diff, or a pull request in Kani. Uses independent parallel lenses for exhaustive frozen-scope discovery, risk-scaled targeted repair verification, and a required self-contained reuse, quality, and efficiency simplification pass. Use when the user invokes $pr-review-toolkit, asks for a comprehensive self-review and fixes, requests a pre-PR quality pass, or wants review findings resolved before publishing. Do not use for a read-only review when the user has not authorized edits.
---

# PR Review Toolkit (Kani)

Review the selected change set through independent lenses, validate every
reported issue against the source, and fix validated in-scope blockers.
Completion is based on coverage, closure evidence, and residual risk — not an
arbitrary number of review rounds and not an unattainable claim of perfect
software.

Explicit invocation of `$pr-review-toolkit`, or an explicit request for this
toolkit, authorizes local edits within the frozen repair boundary and
proportionate verification. It does not authorize commits, pushes,
pull-request writes, issue updates, deployments, architecture expansion, or
other external mutations.

## Non-negotiable contract

1. Preserve the user's scope and unrelated changes.
2. Use independent subagents for broad discovery. Review subagents never edit files.
3. Run applicable discovery lenses concurrently, subject to available concurrency slots, and wait for all of them before editing.
4. Require every discovery reviewer to inspect its entire assigned surface, make the two internal passes defined in the lens contract, and return one complete batch of findings plus a coverage attestation.
5. Reconcile coverage before fixing. Do not mistake repeated broad searches for thoroughness.
6. Validate findings yourself; never apply subagent output blindly.
7. Batch related fixes, then close each repaired invariant with risk-appropriate targeted evidence. Do not automatically restart whole-diff discovery after ordinary repairs.
8. Run all three self-contained simplification lenses once after behavior stabilizes. Fix every validated simplification finding that is behavior-preserving, concrete, narrow, materially useful, and inside the repair boundary.
9. Never recursively rerun all primary or simplification lenses until `CLEAN`. A new broad discovery phase requires a concrete coverage-invalidation trigger and explicit user approval with reason, risk, scope, and expected cost.
10. Finish with `PASS`, `PASS_WITH_RESIDUALS`, or `BLOCKED` according to the evidence-based definition of done.

If subagent tools are unavailable, explain that the required independent
parallel discovery cannot be completed and ask whether to continue with a
degraded local-only pass. Do not silently substitute one blended review.

## Repository bindings — what owns a guarantee in Kani

Kani is a formal verification tool where soundness is paramount ("soundness over
convenience: Kani must never produce false negatives saying code is safe when it isn't").
Guarantees across Kani and its qualification tooling are owned by distinct artifacts:

| Owning artifact | Where it lives | How to run or probe it |
|---|---|---|
| Red-mutation safety probe (fail-closed qualification) | `qualification/fixtures/mutations/`, `tools/kani-qualify/src/mutations.rs` | `cargo run -p kani-qualify -- mutations --fixtures qualification/fixtures/mutations` |
| Parser & model verification receipt truth | `tools/kani-qualify/src/{parser,model,composer,gate}.rs` | `cargo test -p kani-qualify` |
| Verifier end-to-end harness test | `tests/kani/` | `cargo run -p compiletest -- --suite kani --mode kani` |
| Diagnostic & output oracle truth | `tests/expected/`, `tests/ui/` | `cargo run -p compiletest -- --suite expected --mode expected` |
| Cargo integration verification | `tests/cargo-kani/` | `cargo run -p compiletest -- --suite cargo-kani --mode cargo-kani` |
| Core crate unit & property tests | `cprover_bindings`, `kani-compiler`, `kani-driver`, `kani_metadata`, `kani` | `cargo test -p <crate>` |
| License, copyright & supply-chain gate | Root, `.cargo/config.toml`, `deny.toml` | `git ls-files . \| grep -v -E -f scripts/ci/copyright-exclude \| tr '\n' '\0' \| xargs -0 python3 scripts/ci/copyright_check.py && cargo deny check` |
| Code formatting & style | `rustfmt.toml` | `./scripts/kani-fmt.sh --check` |

Unlike downstream leaf models where unit tests are replaced entirely by formal
proofs, Kani is the verification compiler and qualification system itself.
Guarantee coverage in Kani requires:
1. **Soundness**: Verifier logic must never falsely report safety.
2. **Fail-Closed Qualification**: Tools such as `kani-qualify` must fail-closed on any parser ambiguity, missing probe failure, unexpected pass, or backend crash.
3. **Regression Tests**: Every bug fix or new verifier capability must be backed by a test in `tests/kani/`, `tests/expected/`, or crate unit tests (`cargo test -p <crate>`).
4. **License & Copyright Truth**: Dual Apache-2.0 / MIT headers on all new source files.

## Phase 1: Freeze a risk-aware review charter

1. Read the applicable repository instructions (`AGENTS.md`, `CONTRIBUTING.md`).
2. Inspect `git status --short`, current branch, remotes, and the complete diff against the base branch (`main`).
3. Include staged, unstaged, and untracked files. Inspect untracked paths explicitly because Git diffs omit them.
4. Record a review charter containing:
   - base SHA, head SHA, dirty paths, exact review paths, and diff command;
   - acceptance criteria, changed behaviors, and claimed invariants;
   - permitted repair boundary, including directly relevant compiler crates, qualification tooling, and test fixtures;
   - inspection context such as callers, consumers, schemas, and operational contracts;
   - explicit non-goals, known pre-existing conditions, and user-supplied focus;
   - a risk profile for each invariant, using the highest applicable risk for that invariant.
5. Freeze one snapshot for broad discovery. Do not modify files until all discovery reviewers have returned and coverage has been reconciled.

Risk profiles:
- `STANDARD`: local qualification tooling, test harness, CLI commands, or documentation with bounded impact and strong unit test or red-mutation coverage.
- `ELEVATED`: compiler translation, CPROVER IR generation, solver orchestration, unwinding bounds, stubbing logic, or qualification fail-closed safety gate contracts.
- `UNBOUNDED_OR_CROSS_CUTTING`: changes altering soundness semantics across the entire compiler or solver boundary without clear isolation. Requires `NEEDS_USER_DECISION`.

## Phase 2: Exhaustive broad discovery

Run one comprehensive broad discovery phase across the frozen charter. The primary lenses are:

- **General code correctness and compiler soundness**: always run. Focus on logic, soundness, Rust idiomatic patterns, no UB, error propagation.
- **Comment and documentation accuracy**: run when comments, docstrings, docs, or explanatory text changed; check Apache-2.0 / MIT copyright headers on all new files.
- **Guarantee coverage**: run when behavior or tools changed. Map every changed behavior to its owning unit test (`cargo test`), red-mutation probe (`kani-qualify mutations`), or `compiletest` suite. Ensure soundness over convenience.
- **Silent failures and error handling**: run when control flow, process spawning, parser logic, regex parsing, exit codes, or CLI commands changed. Ensure child process failures are never swallowed and unknown states fail-closed.
- **Type design and invariants**: run for concrete models, schemas, serialization types, CLI argument structs, and receipts. Ensure exhaustive matches, domain newtypes, and typed verdicts.
- **Precedent & blast radius**: run for tool additions, workspace `Cargo.toml` modifications, dependency choices, and CI workflow changes. Ensure alignment with Kani conventions.

Spawn one fresh review-only subagent per lens concurrently. Each discovery prompt includes:
- repository path;
- frozen charter, base/head, and exact review paths or diff command;
- applicable project-instruction paths (`AGENTS.md`);
- the shared discovery contract and that reviewer's lens contract from `references/lens-contracts.md`.

Do not include your suspicions or another reviewer's output. Wait for all reviewers before editing. Reconcile coverage against the matrix before moving to repairs.

## Phase 3: Validate, classify, and batch repairs

Create a finding ledger keyed by stable IDs (`CODE-1`, `GUARANTEE-1`, `SILENT-1`, etc.) tracking:
- `origin`: `ORIGINAL_DIFF`, `FIX_REGRESSION`, `DEPENDENT`, or `PREEXISTING`;
- `scope`: `IN_SCOPE` or `SCOPE_EXPANSION`;
- `severity`: `P0`, `P1`, `P2`, or `P3`;
- `disposition`: `BLOCK`, `DEFER`, `REJECT`, or `NEEDS_USER_DECISION`;
- violated acceptance criterion, repository rule, or invariant;
- concrete evidence or counterexample;
- smallest correct fix and required closure proof.

Mark an issue `BLOCK` when its proven defect is in scope and violates soundness, fail-closed behavior, correctness, or repository rules.

Fix all `BLOCK` entries in the parent session as coherent batches. Keep edits narrow and preserve user changes.

## Phase 4: Targeted repair closure

Every accepted repair requires closure evidence against the named finding, invariant, and impacted contracts:
1. Re-read the repair hunk and affected contracts.
2. Run the owning test, probe, or check (e.g. `cargo test -p kani-qualify`, `cargo run -p kani-qualify -- mutations`, `kani-fmt.sh`).
3. For `ELEVATED` risk, spawn a fresh targeted verifier subagent using the contract in `references/lens-contracts.md`.

Behavior is stabilized when all `BLOCK` entries have closure evidence and deterministic checks pass.

## Phase 5: Required self-contained simplification

Run simplification once after behavior stabilizes. Freeze the stabilized diff and spawn three fresh review-only subagents:
- **Reuse**: search for existing standard library or workspace utilities (`regex`, `serde`, `clap`, `anyhow`, `tempfile`) that could replace newly written logic.
- **Quality**: check for redundant state, parameter sprawl, copy-paste blocks, leaky abstractions, unneeded nesting, or useless code-narrating comments. Ensure match arms are exhaustive and sorted alphabetically.
- **Efficiency**: check for unnecessary work, redundant allocations, unbuffered I/O, regex recompilation, or unneeded copies.

Wait for all three before editing. Fix validated findings in one batch. Verify each fix with the owning test suite (`cargo test -p kani-qualify`, `./scripts/kani-fmt.sh`).

## Phase 6: Govern broad-review invalidation

Do not rerun broad review unless a concrete invalidation trigger occurs (e.g., user materially changes scope, repair crosses architecture boundary, or public API changes fundamentally). Ordinary narrow repairs do not invalidate broad discovery.

## Phase 7: Final integrity and definition of done

1. Re-read final repair hunks and check `git status --short`.
2. Confirm no subagent edited files and no user changes were lost.
3. Run all required repository checks:
   - `cargo test -p kani-qualify`
   - `./scripts/kani-fmt.sh --check`
   - `git ls-files . | grep -v -E -f scripts/ci/copyright-exclude | tr '\n' '\0' | xargs -0 python3 scripts/ci/copyright_check.py`
   - `cargo check --tests -p kani-qualify`
4. Reconcile final ledger and coverage matrix.
5. Report final status (`PASS`, `PASS_WITH_RESIDUALS`, or `BLOCKED`).
