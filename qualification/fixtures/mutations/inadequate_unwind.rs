// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(1)]
fn inadequate_unwind() {
    let mut value = 0_u8;
    while value < 2 {
        value += 1;
    }
    // Only an unwinding assertion failure should cause this harness to fail verification
    assert!(value <= 2);
}
