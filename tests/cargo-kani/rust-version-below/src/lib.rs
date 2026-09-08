// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
pub fn check_rust_version_below() {
    let x = kani::any::<u8>();
    kani::assume(x < 100);
    assert!(x + 1 <= 100);
}
