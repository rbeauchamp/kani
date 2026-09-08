<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Publication readback: `0.67.0+qualified.20260908.1`

Status: **published and read back successfully as a controlled-use prerelease**

Publication time: 2026-09-08T19:41:42Z

Release: [`qualified-0.67.0+20260908.1`](https://github.com/rbeauchamp/kani/releases/tag/qualified-0.67.0%2B20260908.1)

GitHub release ID: `385014801`

## Tag readback

| Field | Observed value |
|---|---|
| Tag | `qualified-0.67.0+20260908.1` |
| Annotated tag object | `18a8e896cad4aa26ac0708b89d1ace341c9cf51e` |
| Peeled commit | `734fcd7fcf4b1f8b3e37434d911209e6300da643` |
| Source tree | `ee0a58fd8be2e9e13057d7dbe4de588893d949d3` |
| GitHub signature verification | `verified: true`, reason `valid`, verified at 2026-09-08T19:43:54Z |
| Public metadata commit | `a395ce867` (release-metadata PR #18 merge) |

The tag deliberately does not begin with `kani-`. Kani's upstream-derived
release workflow treats `kani-*` tags as package-version releases and requires
the tag version to equal the Cargo package version. This distribution retains
the upstream-reported package version `0.67.0`, so the separate `qualified-`
namespace avoids invoking an inapplicable release workflow.

## Asset readback

| Asset | Size | GitHub asset ID | GitHub SHA-256 |
|---|---:|---|---|
| `kani-0.67.0+qualified.20260908.1-aarch64-apple-darwin.tar.gz` | 109,764,848 | `551179880` | `0dbd12a6f894fb9a55aa68a9749c4dc4f9da3e3f3d0cbfe7dde69ef2a341c1a6` |
| `kani-0.67.0+qualified.20260908.1-x86_64-unknown-linux-gnu.tar.gz` | 142,938,106 | `551179883` | `f5be1cbe3d3887e4bea9e8d5a2c856e9ae04a4e5d4425ee1cdef957a9960966a` |
| `kani-verifier-0.67.0-734fcd7.crate` | 29,896 | `551179886` | `ac1230b2e0b34e55ecb9e8640e76ead5d63b1d0a3fccb74cbcddf087e9169022` |
| `kani-verifier-0.67.0-734fcd7.sbom.json` | 30,664 | `551179887` | `e1261b6ea5f98573f9d7cd14eee28280dad04078d4236be96d06b6eb926b283c` |
| `kani-0.67.0+20260908.1-aarch64-unknown-linux-gnu-not-qualified.tar.gz` | 141,083,949 | `551179889` | `2c232b74145c81075ca38746a8a35c3205ca717509270b8ca7207b31f2ee6425` |
| `RELEASE-SCOPE.md` | 7,073 | `551179922` | `df9adc9a0becef8d08d9c8a51131152a0b824aa07f8f9f6d518eb264e358d151` |
| `PROVENANCE.json` | 3,944 | `551179924` | `bc9aa02bb12b887b5978500a332e881b72d3fe4132f3a281d5793142994573d9` |
| `SHA256SUMS` | 765 | `551179905` | `9e7e8e8889676d2c90250b9028b9b770da91cec005059097dfcc6d79b06f4633` |
| `SHA256SUMS.asc` | 833 | `551179909` | `0bc2c4f6c20f7f900e7d084ae52f2585b0b2ad722f4aac63952b6a47f39af046` |

The release is public, is marked as a prerelease, and is not a draft. The
`releases/latest` endpoint returns 404 (no full release exists), so the
prerelease is not `Latest`. Repository-level immutable releases are not
enabled, so the release records `immutable: false`; integrity instead rests on
the signed annotated tag, GitHub asset digests, signed `SHA256SUMS`, and this
readback.

## Download and install readback

After publication, all nine assets were downloaded into a new directory.
`shasum -a 256 -c SHA256SUMS` passed for every covered file, the detached
signature verified as a good signature against fingerprint
`2A76 8912 80F2 61EB DBC9 1F66 E530 DB27 2392 A04C`, and the downloaded scope
and provenance files were byte-identical to the staged release inputs.

A **clean install** was then performed from the published assets into a fresh
isolated environment (new `KANI_HOME`/`CARGO_HOME`/install root; the official
Rust 1.97.1 toolchain previously verified against its published `.sha256` —
the toolchain is not a release asset):

- `cargo-kani --version --verbose` matched the toolchain manifest's recorded
  identity exactly (three-line form naming commit `734fcd7`, rustc
  `1.97.1 (8bab26f4f 2026-07-14)`, LLVM 22.1.6, CBMC 6.11.0).
- The full public-corpus gate (`qualification/scripts/core_v1_gate.py`,
  profile `core-v2`) was rerun against the **published** bundle bytes
  (published files renamed to the manifest's artifact names; bytes unchanged):
  **`QUALIFICATION GATE: PASS (public-corpus: 12 harnesses)`** — 12/12
  successful, 3/3 covers, exact unreachable distribution, diagnostics ledger
  exact.
- Linux x86_64: the published asset's GitHub digest
  `f5be1cbe3d3887e4bea9e8d5a2c856e9ae04a4e5d4425ee1cdef957a9960966a` equals the
  artifact hash recorded in the Linux gate receipt
  (`receipts/2026-09-08-linux-x86_64-core-v2.json`), so the bytes that passed
  the Linux gate are the bytes published.

This publication readback establishes artifact transport and identity. The
qualified behavioral scope and limitations remain those in
[`release-scope.md`](release-scope.md); this record does not broaden them.
