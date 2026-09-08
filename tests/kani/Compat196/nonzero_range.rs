// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.96.0 compatibility corpus: iteration over ranges of `NonZero`
//! integers. core implements `Step` for `NonZero<u*>` (core/src/iter/
//! range.rs), which makes `start..end` and `start..=end` iterable when the
//! bounds are non-zero.

extern crate core;

use core::num::NonZeroU32;

/// Concrete range: iterated values are non-zero, ordered, and complete.
#[kani::proof]
#[kani::unwind(5)]
fn check_nonzero_range_concrete() {
    let start = NonZeroU32::new(1).unwrap();
    let end = NonZeroU32::new(4).unwrap();

    let mut seen = [0u32; 3];
    let mut i = 0;
    for x in start..end {
        assert!(x.get() >= start.get() && x.get() < end.get());
        seen[i] = x.get();
        i += 1;
    }
    assert!(i == 3);
    // (element-wise: `seen == [1, 2, 3]` would lower to a memcmp loop)
    assert!(seen[0] == 1 && seen[1] == 2 && seen[2] == 3);

    // inclusive ranges of NonZero work too
    let three = NonZeroU32::new(3).unwrap();
    let mut count = 0;
    let mut last = 0;
    for x in start..=three {
        assert!(x.get() >= 1 && x.get() <= 3);
        last = x.get();
        count += 1;
    }
    assert!(count == 3);
    assert!(last == 3);

    // DoubleEndedIterator via Step::backward_checked
    let mut it = start..end;
    assert_eq!(it.next_back(), NonZeroU32::new(3));
    assert_eq!(it.next(), NonZeroU32::new(1));
    assert_eq!(it.next_back(), NonZeroU32::new(2));
    assert_eq!(it.next(), None);
}

/// Symbolic bounds (kept tiny with `kani::assume`): the iteration is exactly
/// the ascending sequence `start, start+1, ..., end-1`.
#[kani::proof]
#[kani::unwind(7)]
fn check_nonzero_range_symbolic() {
    let a: u32 = kani::any();
    let len: u32 = kani::any();
    kani::assume(a >= 1);
    kani::assume(a <= u32::MAX - 6);
    kani::assume(len <= 5);
    let b = a + len;

    let start = NonZeroU32::new(a).unwrap();
    let end = NonZeroU32::new(b).unwrap();

    let mut expect = a;
    let mut count: u32 = 0;
    let mut prev: u32 = 0;
    for x in start..end {
        assert!(x.get() != 0); // every item is non-zero
        assert!(x.get() == expect); // complete, in order
        if count > 0 {
            assert!(x.get() > prev); // strictly ascending
        }
        prev = x.get();
        expect += 1;
        count += 1;
    }
    assert!(count == len);
    assert!(expect == b);
}
