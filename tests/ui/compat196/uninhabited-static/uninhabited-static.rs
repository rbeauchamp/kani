// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Rust 1.96.0 compatibility: the `uninhabited_static` lint is deny-by-default,
//! so declaring a static of uninhabited type is a compile error. The `extern`
//! formulation triggers exactly the lint denial; an initializing expression
//! (e.g. `transmute`) would fail const evaluation first instead.

#![allow(dead_code)]

enum Void {}

extern "C" {
    static S: Void;
}

#[kani::proof]
fn check() {}
