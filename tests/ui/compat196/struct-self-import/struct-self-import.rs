// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Rust 1.96.0 compatibility: a `{self}` import whose parent is a struct (not a
//! module) is rejected. On 1.97.1 this is reported as
//! `error[E0432]: unresolved import` with "`S` is a struct, not a module".

struct S;

use S::{self as Other};

#[kani::proof]
fn check() {
    let _x: Other = S;
}
