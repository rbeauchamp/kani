<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# core-v2 toolchain-equivalence comparison (macOS ARM64)

Status: **qualification evidence for the core-v2 freeze** (binds the
Phase-2 deferral in `qualification/compat-ledger-rust-1.97.1.md`
"Toolchain-orchestration checks"; required by `profiles/core-v2.md` —
divergence blocks qualification).

Execution date: 2026-09-08

## Protocol

Same frozen consumer source (`qualification/fixtures/public-corpus` in a clean
worktree at `734fcd7fcf4b1f8b3e37434d911209e6300da643`), built and verified
twice with **isolated target directories** (`CARGO_TARGET_DIR` under
`/tmp/core-v2-compare/{target-dev,target-bundle}`) and `Cargo.lock` **deleted
before each arm** so each toolchain regenerates it independently.

| Arm | Kani build | Rust toolchain |
|---|---|---|
| A | Dev build (`target/kani`, `scripts/cargo-kani` wrapper) | Ordinary rustup-managed stable 1.97.1 |
| B | Release bundle `kani-latest-aarch64-apple-darwin.tar.gz` (release run [34259418299](https://github.com/rbeauchamp/kani/actions/runs/34259418299), sha256 `0dbd12a6…341c1a6`), isolated `KANI_HOME`/`CARGO_HOME` | Official `rust-1.97.1-aarch64-apple-darwin.tar.gz` (sha256 `cbd14c36f039f6f11f38148a6295d8234d18ddf20bea53031c86f119423a8b26`, verified against the official `.sha256`), fail-closed identity match with the bundle's `rustc-version` |

Arm A source equivalence: `git diff b7a8c8cf3..734fcd7fc` over
`kani-compiler/ kani-driver/ cprover_bindings/ kani_metadata/ library/ src/`
is **empty** — the dev build implements exactly the frozen Rust source.

Both arms report the same tuple: Kani 0.67.0, `rustc 1.97.1 (8bab26f4f
2026-07-14)`, LLVM 22.1.6, CBMC 6.11.0, kissat 4.0.1.

## Results

| Comparison | Outcome |
|---|---|
| Harness inventory (`cargo kani list --format json`) | **Byte-identical** (12 harnesses) |
| Verification outcomes (canonical 31 lines: per-harness `0 of N failed [(U unreachable)]`, covers, `VERIFICATION:- SUCCESSFUL`, terminal summary, warnings/unsupported ledger) | **Identical** — 12/12 SUCCESS, 0 failures, 3/3 covers, unreachable distribution exact |
| `Cargo.lock` | **Byte-identical**: sha256 `2b10b308692a5ce932ae34503b9b4af3e990d5f929a3d852470c239ddb4d7d04` at all four checkpoints (after list and after verify, both arms) |

Raw logs (`dev-run.log`, `bundle-run.log`, canonical extracts, lock hashes)
were retained under `/tmp/core-v2-compare` at execution time; the values above
are the decision-bearing record.

## What this establishes

The ordinary rustup stable 1.97.1 path and the release-bundle path produce
identical harness inventories, identical verification outcomes, and identical
lockfile behavior for the frozen consumer. No divergence; the profile's
blocking condition is not triggered.

It does not establish equivalence for arbitrary consumers, other platforms, or
future toolchain versions; those remain gated by their own evidence.
