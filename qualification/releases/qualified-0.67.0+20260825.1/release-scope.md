<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Kani `0.67.0+qualified.20260825.1`

Release kind: **unofficial controlled-use GitHub prerelease**

Tag: `qualified-0.67.0+20260825.1`

Profile: [`core-v1`](../../profiles/core-v1.md)

This downstream distribution packages an exact upstream Kani development
revision for conservative, explicitly bounded proof harnesses. It is published
from an unofficial fork and is not sponsored or endorsed by the upstream Kani
team.

Kani exists because of substantial work by its maintainers and contributors.
This distribution is grateful for that work, preserves the upstream-built
artifacts byte for byte, and is intended to make a narrowly qualified revision
available without implying an upstream release or broader guarantee.

## Exact identity

| Field | Value |
|---|---|
| Distribution version | `0.67.0+qualified.20260825.1` |
| Git tag | `qualified-0.67.0+20260825.1` |
| Tag target | `4f7baae414d596eaa82ee90ee529f9957ca565dd` |
| Source tree | `eff9f33ba2a79a3c815ba58080ccd3395955a88d` |
| Upstream repository | `https://github.com/model-checking/kani` |
| Upstream build | [Release Bundle run 32815085848](https://github.com/model-checking/kani/actions/runs/32815085848) |
| Kani-reported version | `Kani Rust Verifier 0.67.0 (4f7baae)` |
| Rust | `nightly-2026-04-01`; commit `48cc71ee8`; LLVM 22.1.2 |
| CBMC | `6.10.0 (cbmc-6.10.0)` |
| Kissat | `4.0.1` |

The Kani executable reports `0.67.0` because the selected upstream development
revision had not changed the package version. The distribution suffix and exact
commit distinguish these artifacts from the official `kani-0.67.0` release.

## Published assets

| Asset | Platform or purpose | SHA-256 |
|---|---|---|
| `kani-0.67.0+qualified.20260825.1-aarch64-apple-darwin.tar.gz` | macOS ARM64 bundle | `4f923fc786f38b830535a4cfddd3714ade173461f82dc85f84eabfecc8fb857a` |
| `kani-0.67.0+qualified.20260825.1-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 bundle | `b8ef05fa132a2d24adad9206e69f3bfa80c9108dd5c3e83495b0f7c2ba2f876f` |
| `kani-verifier-0.67.0-4f7baae.crate` | Exact installer source | `35f5ca813caa2358150fa9417f06cc9d243f3f21f438521dc7a041043530a8ef` |

The two bundles and installer crate were downloaded from the successful
upstream build. Only their release-facing filenames changed; their contents and
hashes did not.

`SHA256SUMS` covers the bundles, installer source, this release-scope record,
and the machine-readable provenance record. `SHA256SUMS.asc` is a detached
signature made by the fork maintainer's Git signing key. The release tag is
also signed. The signing-key fingerprint is
`2A76 8912 80F2 61EB DBC9 1F66 E530 DB27 2392 A04C`; the public key is
available from [`https://github.com/rbeauchamp.gpg`](https://github.com/rbeauchamp.gpg).

## Installation

Download the bundle for the target platform, the installer crate, and
`SHA256SUMS` from this release. Verify the files before installation:

```bash
shasum -a 256 -c SHA256SUMS
```

Extract and install the exact installer source, then direct it to the downloaded
bundle instead of the official release channel:

```bash
mkdir kani-qualified-installer
tar -xzf kani-verifier-0.67.0-4f7baae.crate -C kani-qualified-installer
cargo install --locked \
  --path kani-qualified-installer/kani-verifier-0.67.0

cargo kani setup --use-local-bundle \
  ./kani-0.67.0+qualified.20260825.1-<target>.tar.gz

cargo kani --version --verbose
```

Replace `<target>` with `aarch64-apple-darwin` or
`x86_64-unknown-linux-gnu`. Do not run a later plain `cargo kani setup` for this
installation: that command uses the official release channel and may replace
the qualified bundle.

## Qualified scope

This distribution is intended only for the conservative `core-v1` envelope:

- explicitly named proof harnesses;
- exact harness inventory and terminal-result checking;
- reviewed bounded loops with enforced unwind assertions;
- required cover obligations; and
- the pinned default CBMC and Kissat path.

The complete exclusions remain normative in the
[`core-v1` profile](../../profiles/core-v1.md). Important exclusions include
autoharness, experimental quantifiers, `Rc` or `Arc` nondeterministic
generation, `--quiet`, JSON as a verdict oracle, concurrency claims,
experimental contracts, and any unreviewed warning, unsupported operation,
timeout, `UNDETERMINED`, or unexplained `UNREACHABLE` result.

Each adopting application must retain its own access-controlled manifest and
acceptance receipt. This public distribution does not publish or broaden any
application-specific claim.

## Evidence and limitations

Public evidence includes the exact upstream build and CI results, artifact
hashes, tool identities, result-integrity probes, the fail-closed gate and its
unit tests, the candidate assessment, and the recorded `core-v1` exclusions.

This release does not establish general Kani, rustc, CBMC, solver, or Rust
correctness. It carries no claim for an excluded feature, an unrecorded target,
or a changed source, toolchain, artifact, flag set, harness set, assumption, or
diagnostic ledger. A change to any of those inputs requires requalification.

The release is marked as a GitHub prerelease because the broader public stable
promotion roadmap, including a public corpus, SBOM, and independent public
review, is not complete. The prerelease is nevertheless immutable and usable
for controlled adopters that satisfy their own acceptance gate.
