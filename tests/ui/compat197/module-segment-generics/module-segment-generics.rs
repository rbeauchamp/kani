// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Rust 1.97.0 compatibility: generic arguments on a module path segment are
//! forbidden even when the module reexports a generic enum variant (upstream
//! rust-lang/rust#154962, fixed in rust-lang/rust#154599). On 1.97.1,
//! `m::<u8>::A(1)` is rejected with
//! `error[E0109]: type arguments are not allowed on module`.

pub enum E<T> {
    A(T),
}

mod m {
    pub use super::E::A;
}

#[kani::proof]
fn check() {
    let _x = m::<u8>::A(1);
}
