// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn false_assertion() {
    let x: u8 = kani::any();
    // Reachable failure: the increment overflows when x is 255.
    assert!(x + 1 > x);
}
