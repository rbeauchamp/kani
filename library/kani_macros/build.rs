// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Detects whether the toolchain supports `proc_macro::Diagnostic`, mirroring
//! the condition `proc-macro2-diagnostics` uses to select its emission path.
//! Kani needs the distinction because the stable fallback emits
//! `::core::compile_error!`, which does not resolve under edition 2015 (the
//! default for standalone `kani` invocations); see `emit_diagnostic` in
//! `src/lib.rs`.

fn main() {
    // Make sure `kani_sysroot` is a recognized config
    println!("cargo::rustc-check-cfg=cfg(kani_sysroot)");
    println!("cargo::rustc-check-cfg=cfg(kani_nightly_diagnostics)");
    if let Some((version, channel, _)) = version_check::triple()
        && version.at_least("1.31.0")
        && channel.supports_features()
    {
        println!("cargo:rustc-cfg=kani_nightly_diagnostics");
    }
}
