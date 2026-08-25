<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# macOS ARM64 candidate 1 evidence

Status: **bootstrap evidence; not a final qualification receipt**

Execution date: 2026-08-25

Candidate: `4f7baae414d596eaa82ee90ee529f9957ca565dd`

This public record contains only Kani-distribution evidence. Application-specific
acceptance evidence is deliberately outside this repository.

## Environment and artifact identity

| Field | Observed value |
|---|---|
| Host | macOS 26.5.2, build 25F84, `arm64` |
| Upstream workflow | [Release Bundle 32815085848](https://github.com/model-checking/kani/actions/runs/32815085848) |
| Bundle artifact | `kani-latest-aarch64-apple-darwin.tar.gz` |
| Bundle SHA-256 | `4f923fc786f38b830535a4cfddd3714ade173461f82dc85f84eabfecc8fb857a` |
| Installer artifact | `aarch64-apple-darwin-kani-verifier.crate` |
| Installer SHA-256 | `35f5ca813caa2358150fa9417f06cc9d243f3f21f438521dc7a041043530a8ef` |
| Kani verbose version | `Kani Rust Verifier 0.67.0 (4f7baae)` |
| Rust | `rustc 1.96.0-nightly (48cc71ee8 2026-03-31)`, LLVM 22.1.2 |
| Rust distribution SHA-256 | `e2b2e2fb9688399690a820c012a1a4d65112061634ab5737f3e2915017874bd4` (official checksum verified) |
| CBMC | `6.10.0 (cbmc-6.10.0)` |
| Kissat | `4.0.1` |

The installer, Rust toolchain, Kani setup, and Cargo cache were placed under a
generated `/private/tmp/kani-qualification.*` directory. `KANI_HOME`,
`CARGO_HOME`, the Cargo install root, and the Rust prefix all pointed inside
that directory. The host default toolchain was not used.

## Artifact and result-path probes

| Probe | Expected | Observed |
|---|---|---|
| `tests/kani/Assert/bool_ref.rs` | Successful proof | Exit 0; 1/1 harness; 0/7 checks failed |
| `tests/kani/Assert/multiple_asserts.rs` | Detected assertion failures | Exit 1; 0/1 harness; two assertion checks failed |
| Same failing file with `--quiet` | Must not be trusted | Exit 0 with no output, reproducing issue #4745 |
| Upstream zero-match filter regression | Missing harness must fail before codegen/export | Passed quiet, JSON-export, and multiple-missing-filter variants |

The quiet result confirms that the flag must be mechanically prohibited in
`core-v1`; it is not evidence against the non-quiet result path.

## Executable gate

[`core_v1_gate.py`](../scripts/core_v1_gate.py) checks artifact hashes, Git
commit/tree and cleanliness, runtime tool versions, prohibited flags, exact
harness equality, unique terminal summaries, every successful marker, covers,
unreachable-count distribution, and exact diagnostic sets. Its unit suite also
rejects prohibited flags and partial or ambiguous summaries.

The public repository contains the toolchain manifest and gate implementation,
not any application-specific manifest or receipt. A passing gate whose manifest
has `bootstrap` status is not a promotion verdict.

## What this evidence establishes

It establishes that the exact upstream macOS ARM artifact can be installed with
an isolated exact toolchain, reports the expected source revision and runtime
dependencies, distinguishes an ordinary successful proof from a failing proof
without `--quiet`, and rejects zero-match harness selection.

It does not establish Linux behavior, full Kani regression replay under
downstream custody, mutation completeness, supply-chain attestation, or general
Kani correctness. Those remain promotion gates.
