// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.96.0 compatibility corpus: `From<T> for LazyCell<T, F>` (and the
//! matching `LazyLock` impl), stabilized via `from_wrapper_impls`. The
//! constructed cell starts already initialized, so no closure is involved.
//!
//! Note: `LazyCell::from(v)` / `LazyLock::from(v)` need an explicit type
//! annotation, because the `F` type parameter does not fall back to its
//! `fn() -> T` default during method-call inference (E0283 without it).

extern crate core;

use core::cell::LazyCell;

#[kani::proof]
fn check_lazy_cell_from_concrete() {
    let cell: LazyCell<u32> = LazyCell::from(42u32);
    assert!(*cell == 42);
    // already initialized, so `get` observes the value
    assert_eq!(LazyCell::get(&cell), Some(&42));
}

#[kani::proof]
fn check_lazy_cell_from_symbolic() {
    let v: u64 = kani::any();
    let cell: LazyCell<u64> = LazyCell::from(v);
    assert_eq!(LazyCell::get(&cell), Some(&v));
    assert!(*cell == v);

    // the initialized value is observable and mutable in place
    let mut cell2: LazyCell<u64> = LazyCell::from(v);
    *LazyCell::get_mut(&mut cell2).unwrap() = v.wrapping_add(1);
    assert!(*cell2 == v.wrapping_add(1));
}

/// `LazyLock::from` constructs with `Once::new_complete()`, so dereferencing
/// takes the already-initialized fast path -- no closure, no parking, no
/// threading -- which Kani handles.
#[kani::proof]
fn check_lazy_lock_from() {
    let v: u32 = kani::any();
    let lock: std::sync::LazyLock<u32> = std::sync::LazyLock::from(v);
    assert!(*lock == v);
    // repeated derefs observe the same initialized value
    assert!(*lock == v);
}
