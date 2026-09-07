// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Reported statuses are disjoint subsets of the property inventory.
pub fn counts_fit(selected: u32, undetermined: u32, unreachable: u32, total: u32) -> bool {
    match selected.checked_add(undetermined).and_then(|n| n.checked_add(unreachable)) {
        Some(count) => count <= total,
        None => false,
    }
}
