<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Open soundness work triage for `core-v1`

Status: **bootstrap triage; refresh before promotion**

Snapshot date: 2026-08-25

## Scope and method

This record classifies every open issue carrying Kani's `[F] Soundness` label at
the snapshot time, plus the open soundness-labeled quantifier pull request. The
classification asks only whether the item intersects the narrow `core-v1`
release claim. It is not a judgment about upstream priority, severity outside
that profile, or the quality of the project.

Labels are discovery aids rather than a complete risk inventory. Unlabeled P0
and P1 searches and exact source review remain separate promotion gates.

## Disposition table

| Item | `core-v1` relationship | Bootstrap disposition |
|---|---|---|
| [Issue #4752](https://github.com/model-checking/kani/issues/4752), nondeterministic `Rc`/`Arc` counts | Can support a false property when affected `Arbitrary` values are used | Exclude autoharness and nondeterministic `Rc`/`Arc` generation |
| [PR #4719](https://github.com/model-checking/kani/pull/4719), backend-dropped quantifiers | Can silently remove a symbolic quantified assumption | Exclude experimental quantifiers until a fail-closed fix is integrated and requalified |
| [Issue #3441](https://github.com/model-checking/kani/issues/3441), Rust source-coverage regions | Source-coverage instrumentation can mark code after a divergent call as covered | Exclude source-based coverage; required `kani::cover!` properties remain distinct and must be checked directly |
| [Issue #1150](https://github.com/model-checking/kani/issues/1150), high pointer offsets | The known false-success case is fixed in the candidate by `fb037f320`; exact wrapped-address relations remain incomplete pending backend work | Retain the regression and pointer checks; exclude claims that depend on exact wrapped-pointer equality, ordering, or preserved provenance |
| [Issue #316](https://github.com/model-checking/kani/issues/316), global assembly | Arbitrary injected code cannot be analyzed | Exclude inline and global assembly; the gate must not allow ignore-assembly escape flags |
| [Issue #314](https://github.com/model-checking/kani/issues/314), object aliasing | Rust aliasing violations are outside current detection | Exclude Stacked Borrows, Tree Borrows, and general aliasing-validity claims; require complementary tools where relevant |
| [Issue #310](https://github.com/model-checking/kani/issues/310), CBMC correctness audit | CBMC and its solver path are in every proof's TCB | Pin the backend, run its regressions and red mutations, and retain explicit residual TCB risk; this cannot be described as eliminated |
| [Issue #303](https://github.com/model-checking/kani/issues/303), linking compatibility | Rust and CBMC symbol/link decisions may differ in corner cases | Exclude custom symbol overriding and duplicate/missing-symbol claims; keep linking in the TCB and exercise the exact public corpus |
| [Issue #302](https://github.com/model-checking/kani/issues/302), CBMC serialization | A malformed or version-drifted Irep can alter the generated GOTO program | Pin Kani and CBMC together, run well-formedness/regression gates, and retain serialization in the TCB |
| [Issue #299](https://github.com/model-checking/kani/issues/299), vtable generation audit | No specific current defect is stated, but dynamic dispatch remains a complex audited boundary | Exclude dynamic trait-object and vtable-semantic claims from the first profile |
| [Issue #298](https://github.com/model-checking/kani/issues/298), rustc MIR hiding UB | rustc is a trusted translation stage and may erase source-level distinctions | Keep rustc and its exact flags in the TCB, prohibit unreviewed optimization/configuration drift, and require complementary evidence for unsafe code |
| [Issue #297](https://github.com/model-checking/kani/issues/297), implementation-defined layout | Layout-dependent transmute, FFI, and pointer code can assume facts Rust does not guarantee | Exclude claims based on unspecified `repr(Rust)` layout, padding, field order, or ABI |

## Consequences for the first release

The labeled set does not require clearing the entire soundness backlog before a
restricted release. It does require mechanical exclusions, a public corpus that
does not exercise excluded semantics, and an explicit TCB acceptance decision
for rustc, Kani serialization/linking, CBMC, and the selected solver.

The release remains blocked until the unlabeled P0/P1 search, public feature
classification, mutation gate, independent exact-head review, and platform
receipts are complete.
