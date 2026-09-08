// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Rust 1.97.0 compatibility: tuple-index shorthand is no longer accepted in
//! struct patterns; the field must be bound explicitly (`0: x`). On 1.97.1 the
//! parser rejects `let P { 0 } = p;` with "expected identifier, found `0`".

struct P(u8, u8);

fn f(p: P) {
    let P { 0 } = p;
}

#[kani::proof]
fn check() {
    f(P(1, 2));
}
