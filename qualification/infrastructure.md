<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Qualification infrastructure contract

`kani-qualify` admits verification evidence and exercises rejection paths. A
passing infrastructure run is not a release qualification. Release promotion
also requires the complete profile, artifact provenance, platform matrix,
independent review, and publication controls in the [operating model](operating-model.md).
Development is tracked in [#11](https://github.com/rbeauchamp/kani/issues/11).

## Commands and claims

```sh
# Syntax inspection, including incomplete logs; exit zero is not qualification.
cargo run -p kani-qualify -- parse --log verification.log

# Development infrastructure exercise; no pinned manifest claim.
cargo run -p kani-qualify -- mutation-self-test \
  --fixtures qualification/fixtures/mutations \
  --kani-bin target/kani/bin/kani-driver \
  --receipt mutation-self-test.json

# Reject mismatched runtime identities before executing any proof fixture.
cargo run -p kani-qualify -- mutations \
  --toolchain qualification/manifests/core-v1/toolchain.json \
  --fixtures qualification/fixtures/mutations \
  --kani-bin /absolute/path/to/unpacked-kani/bin/kani-driver \
  --receipt qualification-mutations.json

# Admit producer records; a raw log cannot establish successful termination.
cargo run -p kani-qualify -- compose \
  --toolchain qualification/manifests/core-v1/toolchain.json \
  --consumer consumer.json --runs runs.json --receipt composite.json
```

Mutation receipt schema 2 distinguishes `infrastructure_self_test` (null profile
and toolchain digest) from `qualification_mutations` (the supplied manifest's
profile and byte digest). Composite schema 2 retains the consumer's status; a
`bootstrap` consumer remains bootstrap even when its evidence passes.

## Producer evidence boundary

`runs.json` is an array of strict
[`InputRunEvidence`](../tools/kani-qualify/src/model.rs) records. Each record
requires a unique `run_id`, absolute `cwd`, selected `expected_harnesses`, actual
effective `argv`, observed `exit_code`, optional `terminating_signal`, `log_path`,
and the log's `sha256`. Its required `context` contains source commit/tree,
consumer and toolchain manifest byte digests, declared platform, and observed
runtime versions and executable digests. Missing or unknown fields are rejected.

The producer must capture all stdout and stderr through EOF, retain the actual
terminal state, and bind execution to the recorded source checkout, configuration,
and artifacts. It must account for implicit options from environment, Cargo
configuration, and package metadata when attesting the effective invocation.
The admitted command is `cargo [--config <cwd/config>] kani --manifest-path
<cwd/manifest> --exact --output-format=terse`, followed by one `--harness <name>`
for each selected harness in the recorded order. Additional flags are rejected.

Composition validates supplied evidence; it does not authenticate a producer or
reconstruct a past process. The producer, source-to-build provenance, relationship
between runtime executable hashes and manifest archive hashes, and the recorded
configuration are trusted and require independent review. Fabricated metadata
cannot be distinguished from authentic metadata by a local JSON composer.

Every run must terminate normally with exit zero, match both manifests and the
same runtime/platform context, and report exactly its selected harnesses. The
union must equal the consumer's inventory. Retries are accepted only when every
normalized result field agrees. Each log is read once; those same bytes are
hashed and parsed. The removed `compose --logs` shortcut cannot manufacture
missing process evidence.

## Complete result admission

The parser accepts the sequential terse format. It requires exactly one failure
summary and terminal verdict per harness, at most one cover summary, and one
completion summary consistent with all terminal harness records. Unknown,
duplicate, misplaced, malformed, and contradictory result records reject
admission. Parallel interleaved output and `should_panic` verdict annotations are
outside this format. `parse` can inspect incomplete syntax; composition and
mutation acceptance additionally require completion.

All counts must fit `u32`. For the renderer's disjoint property statuses,
`selected + undetermined + unreachable <= total` is checked without machine
overflow. Undetermined checks can describe failed negative probes but cannot
produce a passing harness. Every harness must satisfy its own covers; an
overcount in one harness cannot cancel a deficit in another. The receipt keeps
all decision-bearing counts. Unsupported-construct blocks are consumed in full,
including rendered type names, and warnings must match the consumer's ledger.

[`result_counts.rs`](proofs/result_counts.rs) compiles the production count guard
directly and verifies its equivalence to a wider-integer mathematical sum for
every combination of four `u32` values. This establishes that guard's arithmetic
contract, subject to the Kani/compiler/solver trusted base. It does not prove the
text parser, OS process execution, provenance, or the verifier itself correct.

## Runtime and publication boundaries

Mutation execution selects one built or unpacked layout containing `kani-driver`
and `kani-compiler`. Backend siblings take precedence; otherwise explicit `PATH`
resolution selects CBMC and Kissat. Private executable links bind those selected
backends to the names the driver invokes. Version queries require normal exit
zero, nonempty stdout, empty stderr, and a 30-second deadline. Driver, compiler,
and backend hashes are checked before and after the probes. The five fixture
sources are copied to a private directory; both source and execution copies are
hashed and checked for drift.

The runner returns output only after normal termination and EOF on both pipes.
Timeouts, signals, decoding errors, and incomplete collection reject execution.
Cancellation kills the process group and reaps the root. This supports Unix
Kani/CBMC tools that wait for their children and do not daemonize. It is not an OS
sandbox: a descendant that escapes the group or deliberately closes inherited
pipes can outlive cleanup. Such processes are outside the supported execution
contract; inherited pipes that remain open still cause rejection. Deadlines
govern admission, while scheduling and cleanup also rely on the OS and signal
delivery.

Negative probes require their expected normal exit behavior and diagnostics;
a crash is not successful detection. Both output streams contribute to parsing
and are hashed in the receipt. The fixed mutation warning ledger records Kani's
injected `register_tool` warning and rustc's one-warning summary; other warnings
or unsupported constructs reject harness probes. Backend rejection retains the
normal failing status and both stream hashes.

The gate executable must be hashable; failure never substitutes a label digest.
Receipt publication stages and synchronizes complete bytes in the destination
directory, then uses no-clobber publication. Existing files, including failed
receipts, are never overwritten by this tool. Use a new path for each attempt.
This is a publication invariant, not filesystem access control or a guarantee
against later privileged modification. A release must preserve and authenticate
its receipts under the operating model's storage and signature controls.
