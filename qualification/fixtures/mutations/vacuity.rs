// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn vacuity() {
    kani::assume(false);
    kani::cover!(true, "the harness has a reachable state");
    assert!(false);
}
