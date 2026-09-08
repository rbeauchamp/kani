// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.97.0 compatibility corpus: the well-known
//! `cfg(target_has_atomic_primitive_alignment)`. On the host target
//! (aarch64-apple-darwin) it carries the values 8, 16, 32, 64, 128 and
//! "ptr" (confirmed via `rustc --print cfg`).

extern crate core;

/// The "ptr" and "64" values hold on every 64-bit Kani platform.
#[kani::proof]
fn check_atomic_primitive_alignment_ptr() {
    assert!(cfg!(target_has_atomic_primitive_alignment = "ptr"));
    assert!(cfg!(target_has_atomic_primitive_alignment = "64"));
    // reachability check: this cover is satisfiable iff the cfg holds
    kani::cover!(cfg!(target_has_atomic_primitive_alignment = "ptr"));
}

/// The values common to the x86_64 and aarch64 targets Kani runs on. Note
/// the target differential: "128" is set on aarch64-apple-darwin but not on
/// x86_64-unknown-linux-gnu (no 128-bit atomics at the x86-64 baseline), so
/// it is checked per-arch below rather than here.
#[kani::proof]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn check_atomic_primitive_alignment_values() {
    assert!(cfg!(target_has_atomic_primitive_alignment = "8"));
    assert!(cfg!(target_has_atomic_primitive_alignment = "16"));
    assert!(cfg!(target_has_atomic_primitive_alignment = "32"));
    assert!(cfg!(target_has_atomic_primitive_alignment = "64"));
    assert!(cfg!(target_has_atomic_primitive_alignment = "ptr"));
}

/// aarch64-apple-darwin additionally carries "128" (confirmed via
/// `rustc --print cfg`); x86_64-unknown-linux-gnu does not.
#[kani::proof]
#[cfg(target_arch = "aarch64")]
fn check_atomic_primitive_alignment_128() {
    assert!(cfg!(target_has_atomic_primitive_alignment = "128"));
}

/// Under the "64" value, `AtomicU64` has the same alignment as the
/// primitive `u64`.
#[kani::proof]
#[cfg(target_has_atomic_primitive_alignment = "64")]
fn check_atomic_u64_matches_primitive_alignment() {
    assert_eq!(
        core::mem::align_of::<core::sync::atomic::AtomicU64>(),
        core::mem::align_of::<u64>()
    );
}
