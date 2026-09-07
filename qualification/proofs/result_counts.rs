// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Compile the production guard itself; the specification uses a wider domain
// where the sum of three u32 values cannot overflow.
#[path = "../../tools/kani-qualify/src/counts.rs"]
mod counts;

#[kani::proof]
fn result_counts_match_mathematical_partition() {
    let selected: u32 = kani::any();
    let undetermined: u32 = kani::any();
    let unreachable: u32 = kani::any();
    let total: u32 = kani::any();
    assert_eq!(
        counts::counts_fit(selected, undetermined, unreachable, total),
        u64::from(selected) + u64::from(undetermined) + u64::from(unreachable) <= u64::from(total),
    );
}
