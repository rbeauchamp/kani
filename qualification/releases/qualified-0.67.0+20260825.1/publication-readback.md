<!-- Copyright Kani Contributors -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Publication readback: `0.67.0+qualified.20260825.1`

Status: **published and read back successfully as a controlled-use prerelease**

Publication time: 2026-08-25T19:16:09Z

Release: [`qualified-0.67.0+20260825.1`](https://github.com/rbeauchamp/kani/releases/tag/qualified-0.67.0%2B20260825.1)

GitHub release ID: `RE_kwDOUD64xs4Wcx2U`

## Tag readback

| Field | Observed value |
|---|---|
| Tag | `qualified-0.67.0+20260825.1` |
| Annotated tag object | `3d2bcd4b875aa7cef896754cbaa037c2dd58dbef` |
| Peeled commit | `4f7baae414d596eaa82ee90ee529f9957ca565dd` |
| Source tree | `eff9f33ba2a79a3c815ba58080ccd3395955a88d` |
| GitHub signature verification | `verified: true`, reason `valid` |
| Verification time | 2026-08-25T19:16:19Z |
| Public metadata commit | `167444876e0dc7888abb173ae360ed8ee23808e2` |

The tag deliberately does not begin with `kani-`. Kani's upstream-derived
release workflow treats `kani-*` tags as package-version releases and requires
the tag version to equal the Cargo package version. This distribution retains
the upstream-reported package version `0.67.0`, so the separate `qualified-`
namespace avoids invoking an inapplicable release workflow.

## Asset readback

| Asset | Size | GitHub asset ID | GitHub SHA-256 |
|---|---:|---|---|
| `kani-0.67.0+qualified.20260825.1-aarch64-apple-darwin.tar.gz` | 105,197,910 | `RA_kwDOUD64xs4fkdq0` | `4f923fc786f38b830535a4cfddd3714ade173461f82dc85f84eabfecc8fb857a` |
| `kani-0.67.0+qualified.20260825.1-x86_64-unknown-linux-gnu.tar.gz` | 142,436,693 | `RA_kwDOUD64xs4fkdqz` | `b8ef05fa132a2d24adad9206e69f3bfa80c9108dd5c3e83495b0f7c2ba2f876f` |
| `kani-verifier-0.67.0-4f7baae.crate` | 29,215 | `RA_kwDOUD64xs4fkdq1` | `35f5ca813caa2358150fa9417f06cc9d243f3f21f438521dc7a041043530a8ef` |
| `PROVENANCE.json` | 2,917 | `RA_kwDOUD64xs4fkdq3` | `1358e91f42f5bb4bd7bf3c8799a18c5b03d44ae98ea90da9e7bcc9c2e3c2e282` |
| `RELEASE-SCOPE.md` | 5,490 | `RA_kwDOUD64xs4fkfIL` | `438173c4940e0cca99aa6aaf6a0770801c10ee0306178c640ee954665e89e72d` |
| `SHA256SUMS` | 524 | `RA_kwDOUD64xs4fkfII` | `b04d3ec0dfbca1ba73b6c7b275161c20a0d2e148559b47da284098bbc969540f` |
| `SHA256SUMS.asc` | 833 | `RA_kwDOUD64xs4fkfIJ` | `6d6a43ad9bb8e30720ee57a3ea1ac68dbc1b0d9ee845cf74b2adea0826dbaef1` |

The release is public, is marked as a prerelease, is not marked `Latest`, and
is not a draft. Repository-level immutable releases are not enabled, so the
release records `isImmutable: false`; integrity instead rests on the signed
annotated tag, GitHub asset digests, signed `SHA256SUMS`, and this readback.

After publication, all seven assets were downloaded into a new directory.
`shasum -a 256 -c SHA256SUMS` passed for every covered file, the detached
signature verified against fingerprint
`2A76 8912 80F2 61EB DBC9 1F66 E530 DB27 2392 A04C`, and the downloaded scope
and provenance files were byte-identical to the staged release inputs.

This publication readback establishes artifact transport and identity. The
qualified behavioral scope and limitations remain those in
[`release-scope.md`](release-scope.md); this record does not broaden them.
