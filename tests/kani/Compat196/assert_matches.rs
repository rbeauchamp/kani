// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.96.0 compatibility corpus: `core::assert_matches!` and
//! `core::debug_assert_matches!`. These macros live at the crate root and
//! are NOT in the prelude, so they need an explicit `use`.

extern crate core;

use core::assert_matches;
use core::debug_assert_matches;

#[derive(Debug)]
enum Shape {
    Point,
    Rect(u32, u32),
}

/// Positive cases: plain patterns, or-patterns, guards, the custom-message
/// form, and `debug_assert_matches!` (active under Kani's debug-assertions
/// build).
#[kani::proof]
fn check_assert_matches_positive() {
    let x: u32 = kani::any();
    kani::assume(x > 4);

    let opt = Some(x);
    assert_matches!(opt, Some(v) if v > 4);

    let res: Result<u32, u8> = if x > 100 { Ok(1) } else { Err(7) };
    assert_matches!(res, Ok(1) | Err(7));

    let s = if x % 2 == 0 { Shape::Point } else { Shape::Rect(x, x) };
    assert_matches!(s, Shape::Point | Shape::Rect(_, _));

    // form with a custom panic message
    assert_matches!(opt, Some(v) if v > 4, "expected a value above 4, got {:?}", opt);

    debug_assert_matches!(opt, Some(v) if v > 4);
}

/// Negative case: a mismatched value makes `assert_matches!` panic.
#[kani::proof]
#[kani::should_panic]
fn check_assert_matches_panics_on_mismatch() {
    let opt: Option<u32> = None;
    assert_matches!(opt, Some(_));
}
