// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Rust 1.97.0 compatibility: `#[export_name = ""]` is now a compile error
//! ("`export_name` may not be empty").

#[export_name = ""]
pub fn exported() {}

#[kani::proof]
fn check() {
    exported();
}
