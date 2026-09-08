// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.96.0 compatibility corpus: the `core::range` module (RFC 3550,
//! "new range types"). Unlike the legacy `core::ops` ranges (which are
//! themselves iterators), `core::range::Range` / `RangeFrom` /
//! `RangeToInclusive` are `Copy` value types that implement `IntoIterator`;
//! iteration goes through the separate `RangeIter` / `RangeFromIter` types.
//! (`RangeToInclusive` has no iterator type: it is unbounded below, so no
//! `RangeToInclusiveIter` exists in core 1.97.1.)

extern crate core;

use core::range::{Range, RangeFrom, RangeToInclusive};

/// Constructing from a legacy range and iterating yields the expected
/// element sequence.
#[kani::proof]
#[kani::unwind(6)]
fn check_range_concrete_sequence() {
    let r = Range::from(1..5u32);
    let mut expect: u32 = 1;
    let mut count: usize = 0;
    for x in r {
        assert!(x == expect);
        expect += 1;
        count += 1;
    }
    assert!(count == 4);

    // `RangeIter` is a fully-fledged iterator: DoubleEnded + ExactSize.
    // (ExactSizeIterator is implemented for usize/u8/u16/isize/i8/i16.)
    let mut it = Range::from(1..5u16).into_iter();
    assert_eq!(it.len(), 4);
    assert_eq!(it.next(), Some(1));
    assert_eq!(it.next_back(), Some(4));
    assert_eq!(it.len(), 2);
    assert_eq!(it.next(), Some(2));
    assert_eq!(it.next(), Some(3));
    assert_eq!(it.next(), None);

    // `IntoIterator` by value, as in the std doc example
    assert_eq!(Range::from(3..6).into_iter().sum::<i32>(), 12);
}

/// `Range` is `Copy`: assignment copies, and `into_iter` (by value) leaves
/// the original usable.
#[kani::proof]
fn check_range_copy_semantics() {
    let r = Range::from(1..5u32);
    let r2 = r; // Copy, not move
    assert_eq!(r, r2);
    assert_eq!(r.start, 1);
    assert_eq!(r.end, 5);

    let s1: u32 = r.into_iter().sum(); // consumes a copy
    let s2: u32 = r2.iter().sum();
    assert_eq!(s1, 10);
    assert_eq!(s1, s2);

    // struct-literal construction and derived PartialEq
    assert_eq!(r, Range { start: 1, end: 5 });
}

/// Symbolic bounds: iteration over `Range::from(a..b)` yields exactly
/// `a, a+1, ..., b-1`.
#[kani::proof]
#[kani::unwind(6)]
fn check_range_symbolic_bounds() {
    let a: u8 = kani::any();
    let b: u8 = kani::any();
    kani::assume(a <= b);
    kani::assume(b - a <= 4);

    let r = Range::from(a..b);
    let mut expect = a;
    let mut count: u8 = 0;
    for x in r {
        assert!(x == expect);
        expect += 1;
        count += 1;
    }
    assert!(expect == b);
    assert!(count == b - a);
}

/// Bidirectional conversion with the legacy `core::ops::Range`.
#[kani::proof]
fn check_range_legacy_conversion() {
    let a: u16 = kani::any();
    let b: u16 = kani::any();

    let r = Range::from(a..b);
    let legacy: core::ops::Range<u16> = r.into();
    assert_eq!(legacy.start, a);
    assert_eq!(legacy.end, b);

    let back: Range<u16> = Range::from(legacy);
    assert_eq!(back, r);

    let legacy2: core::ops::Range<u16> = core::ops::Range::from(r);
    assert_eq!(legacy2, a..b);
}

/// `RangeFrom` (the `start..` shape): `Copy`, `contains`, and iteration via
/// `RangeFromIter`.
#[kani::proof]
fn check_range_from() {
    let start: u16 = kani::any();
    kani::assume(start <= u16::MAX - 3);

    let rf = RangeFrom::from(start..);
    let rf2 = rf; // Copy
    assert_eq!(rf, rf2);
    assert_eq!(rf.start, start);

    assert!(rf.contains(&start));
    assert!(rf.contains(&(start + 2)));
    if start > 0 {
        assert!(!rf.contains(&(start - 1)));
    }

    let mut it = rf.iter();
    assert_eq!(it.next(), Some(start));
    assert_eq!(it.next(), Some(start + 1));
    assert_eq!(it.next(), Some(start + 2));

    // conversion with the legacy `core::ops::RangeFrom`
    let legacy: core::ops::RangeFrom<u16> = rf2.into();
    assert_eq!(legacy.start, start);
    let back: RangeFrom<u16> = RangeFrom::from(legacy);
    assert_eq!(back, rf2);
}

/// `RangeToInclusive` (the `..=last` shape): `Copy` and `contains`, plus
/// conversion with the legacy `core::ops::RangeToInclusive`.
#[kani::proof]
fn check_range_to_inclusive() {
    let last: u8 = kani::any();

    let rti = RangeToInclusive::from(..=last);
    let rti2 = rti; // Copy
    assert_eq!(rti, rti2);
    assert_eq!(rti.last, last);

    assert!(rti.contains(&last));
    assert!(rti.contains(&0));
    if last < u8::MAX {
        assert!(!rti.contains(&(last + 1)));
    }

    let legacy: core::ops::RangeToInclusive<u8> = rti.into();
    assert_eq!(legacy.end, last);
    let back: RangeToInclusive<u8> = RangeToInclusive::from(legacy);
    assert_eq!(back, rti2);
}
