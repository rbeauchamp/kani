<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Operating model

This document is normative. “Must” identifies a promotion requirement; “should”
identifies the default unless a release receipt records a reviewed exception.

## Principles

1. **Upstream-first and respectful.** Generally useful fixes must be prepared as
   focused upstream contributions. Public communication must be appreciative,
   factual, and free of speculation about motives or priorities.
2. **Soundness before convenience.** A false success is more serious than a
   crash, false failure, performance regression, or missing feature.
3. **Exact identity.** Every claim must bind source, dependency, configuration,
   platform, workflow, and proof-corpus identities.
4. **Restricted profile.** A release is qualified only for named behavior. New,
   experimental, unsupported, or unexamined behavior is outside the envelope.
5. **Fail closed.** Missing harnesses, ambiguous output, unreviewed diagnostics,
   unsupported constructs, timeouts, solver errors, `UNDETERMINED`, and
   unexplained `UNREACHABLE` results fail promotion.
6. **Independent review.** The authoring context cannot be the sole authority
   for semantic correctness or promotion.
7. **Minimal downstream delta.** Carry a patch only when required for the active
   profile, and remove it after upstream integration and requalification.
8. **Qualification, not universal correctness.** Evidence must use the strongest
   accurate claim and preserve the explicit trusted computing base.

## Repository and branch topology

- `main` mirrors the upstream `main` history. Fork-specific development does not
  land directly on it.
- `qualification/*` branches contain qualification-program infrastructure and
  records intended for review before integration.
- `candidate/<version>` freezes one proposed release tuple. Upstream intake stops
  after freeze unless a recorded blocker requires rebasing and invalidating all
  prior receipts.
- `fix/<issue>-<slug>` contains one focused fix or hardening change intended for
  upstream contribution.
- signed release tags identify promoted candidate commits. The provisional
  downstream version form is `0.67.0+qualified.YYYYMMDD.N`; it must be validated
  against Cargo and the release workflow before use.

Upstream changes merge into `main`; they do not merge directly into a frozen
candidate. A new upstream base creates a new candidate identity and requires a
fresh gate.

## Roles and separation

| Role | Responsibility | Separation rule |
|---|---|---|
| Program owner | Approves profiles, risk acceptance, and promotion | Remains accountable even when AI performs work |
| Patch author | Produces reducer, fix, regression, and upstream-ready change | Cannot provide the only semantic approval |
| Adversarial reviewer | Tries to falsify the claim at the frozen head | Must use an independent context and inspect the full relevant path |
| Qualification runner | Executes the recorded gate and stores receipts | Must not silently change the candidate or commands |
| Release custodian | Signs, publishes, verifies, and can roll back artifacts | Must verify candidate identity immediately before and after publication |

One person may hold several roles, but authoring and adversarial review evidence
must remain separate. AI agents may perform bounded roles; the program owner
retains promotion authority.

## Risk taxonomy

- **P0 — false success or result corruption:** ignored assumptions/assertions,
  unsound translation/modeling, vacuity presented as proof, missing results,
  incorrect exit status, or backend failure interpreted as success.
- **P1 — semantic and TCB drift:** wrong supported-profile behavior, runtime
  dependency mismatch, architecture divergence, or toolchain changes that alter
  proof meaning.
- **P2 — blocking false failure:** crash, spurious counterexample, nontermination,
  or performance cliff that prevents a required downstream proof.
- **P3 — out-of-profile feature, UX, or maintenance work.**

Labels are discovery aids, not authority. An unlabeled report can still be P0.

## Intake and patch lifecycle

For every candidate issue or patch:

1. Determine whether it intersects an active profile.
2. Reproduce it at the exact released version and exact candidate head.
3. Reduce the reproducer without weakening the failure.
4. State the expected verifier behavior and the unsafe alternative.
5. Add a regression that fails before and passes after.
6. Add a nearby negative mutation that must be rejected.
7. Inspect the relevant compiler, model, driver, backend, and result-processing
   path rather than relying only on CLI output.
8. Obtain adversarial review at the frozen patch head.
9. Run the risk-scaled local and remote gates.
10. Submit a focused upstream pull request when the change is generally useful.
11. Carry it downstream only while required; remove it after the upstream merge
    is incorporated and the new candidate is requalified.

Sensitive security issues follow [Kani's security policy](../.github/SECURITY.md)
and are never developed first in a public issue.

## Profile contents

Every profile must define:

- the public corpus, exact source identities, crates, and claims;
- expected proof harness names and counts;
- allowed Rust features and Kani APIs;
- prohibited, experimental, or unexamined features;
- target OS and architectures;
- exact Kani flags, solver, unwind policy, stubs, contracts, and assumptions;
- reachability and cover obligations;
- accepted result statuses and exit behavior;
- complementary evidence such as Miri, sanitizers, fuzzing, or proof tools;
- the complete TCB and known-limitations ledger; and
- promotion and invalidation conditions.

Unlisted behavior is outside the profile.

Application-specific acceptance records follow the same identity and result
rules but remain in the adopting application's own access-controlled system.
They cannot broaden the public distribution's qualified envelope.

## Qualification gate

A frozen candidate must pass all applicable steps in this order:

1. **Identity gate:** clean canonical checkout; expected branch/tag; exact commit
   and tree; expected upstream base and downstream patches; no replacement refs,
   alternate index, or unexpected Git environment overrides.
2. **Dependency gate:** exact Rust, standard library, Charon, CBMC, solver,
   target, build image, flags, and dependency lock identities.
3. **Source gate:** format, lint, dependency policy, unit tests, regression suites,
   documentation, and release-bundle tests.
4. **Profile gate:** enumerate expected downstream harnesses, require an exact
   set match, run every harness, and reject partial execution.
5. **Reachability gate:** require declared cover obligations and explain every
   unreachable check. Success without the declared reachable state space fails.
6. **Mutation gate:** run profile-specific red probes that deliberately break
   assertions, assumptions, unwind adequacy, harness selection, backend status,
   and result parsing. Every mutation must be detected.
7. **Independent review gate:** reconcile theorem/claim, Rust semantics, Kani
   encoding, backend result, and user-facing conclusion at the exact head.
8. **Artifact gate:** build on every qualified platform, install from the built
   bundle in a clean environment, rerun smoke and profile checks, generate SBOM
   and hashes, and sign/attest the artifacts.
9. **Publication gate:** verify tag, commit, tree, artifact hashes, receipts, and
   release text before publishing; read them back afterwards.

Upstream CI can satisfy portions of step 3 when it is bound to the exact
candidate and the receipt records the run IDs and job conclusions. It cannot
replace steps 4–9.

## Mandatory result policy

- `--quiet` is prohibited until its failure exit behavior is fixed and
  requalified.
- `--export-json` is not a verdict oracle until empty, failed, partial, and
  fail-fast behavior is fixed and requalified.
- Experimental quantifiers, autoharness, nondeterministic `Rc`/`Arc`, loop
  contracts, `--restrict-vtable`, inline/global assembly, concurrency, and
  uninitialized/valid-value experimental checks are excluded from `core-v1`
  unless individually promoted into a later profile.
- Runtime CBMC and solver versions must match the release manifest before any
  harness runs.
- Unwinding assertions must remain enabled and pass.
- Expected harness names and counts must be checked independently of Kani's
  overall exit code.
- Every warning and unsupported-operation diagnostic must match an exact,
  reviewed profile ledger that records its origin, release impact, and
  reachability disposition. A new, missing, or changed diagnostic is fatal;
  ledger membership is not permission to ignore it.
- Cross-platform or cross-solver disagreement quarantines the affected claim
  until explained.

## Release manifest and receipt

Every release must record:

- release name, status, time, and program owner;
- Kani base SHA/tree and ordered downstream patch SHAs/trees;
- source archive and submodule identities;
- Rust, standard library, Charon, CBMC, solver, OS, architecture, image, flags,
  and environment;
- profile version and exact downstream consumer SHAs;
- expected and observed harness manifests;
- upstream and downstream gate commands, run IDs, logs, and terminal results;
- mutation inventory and results;
- assumptions, exclusions, unresolved issues, and reviewed exceptions;
- artifact filenames, sizes, SBOMs, cryptographic hashes, and signatures or
  attestations; and
- rollback target and publication readback.

Receipts are immutable. A correction creates a superseding receipt rather than
editing history silently.

## Upstream synchronization and divergence trigger

The default action is to collaborate upstream. Reconsider a long-lived semantic
fork only when measured evidence shows one or more of the following:

- required P0/P1 fixes are repeatedly rejected or remain unreviewed beyond the
  program's declared risk window;
- required semantics intentionally differ from upstream direction;
- the downstream patch set is no longer small enough to audit and continuously
  rebase; or
- upstream release/security interfaces cannot support the declared profile.

Such a decision requires a new ADR, an explicit maintenance budget, an
accountable verifier/toolchain maintainer, and independent assurance ownership.
