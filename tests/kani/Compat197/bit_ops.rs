// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.97.0 compatibility corpus: integer methods `highest_one`,
//! `lowest_one`, `isolate_highest_one`, `isolate_lowest_one`, `bit_width`
//! and their `NonZero` variants.
//!
//! Semantics (from core/src/num/{int,uint}_macros.rs and num/nonzero.rs):
//!   - `highest_one()`  = index of the most significant set bit, `None` at 0
//!                        (signed: computed on the `as unsigned` bit pattern)
//!   - `lowest_one()`   = index of the least significant set bit, `None` at 0
//!   - `isolate_highest_one()` = value with only the top set bit kept, 0 at 0
//!   - `isolate_lowest_one()`  = `self & self.wrapping_neg()`, 0 at 0
//!   - `bit_width()`    = `BITS - leading_zeros()` (unsigned types only),
//!                        `NonZero<u32>`-valued for `NonZero`
//!
//! Every method is checked against an independent relational specification
//! built from older stable operations (`leading_zeros`, `trailing_zeros`,
//! shifts, `wrapping_neg`) on fully symbolic inputs.

extern crate core;

use core::num::NonZeroU32;

/// Doc-example and edge-case checks (0, 1, MAX, MIN, powers of two).
#[kani::proof]
fn check_concrete_edges() {
    // Examples straight from the std docs
    assert_eq!(0b_01100100u8.isolate_highest_one(), 0b_01000000);
    assert_eq!(0u8.isolate_highest_one(), 0);
    assert_eq!(0b_01100100u8.isolate_lowest_one(), 0b_00000100);
    assert_eq!(0u8.isolate_lowest_one(), 0);
    assert_eq!(0u8.highest_one(), None);
    assert_eq!(0b1u8.highest_one(), Some(0));
    assert_eq!(0b1_0000u8.highest_one(), Some(4));
    assert_eq!(0b1_1111u8.highest_one(), Some(4));
    assert_eq!(0u8.lowest_one(), None);
    assert_eq!(0b1u8.lowest_one(), Some(0));
    assert_eq!(0b1_0000u8.lowest_one(), Some(4));
    assert_eq!(0b1_1111u8.lowest_one(), Some(0));

    // bit_width: 0 at 0, BITS at MAX
    assert_eq!(0u32.bit_width(), 0);
    assert_eq!(0b111u32.bit_width(), 3);
    assert_eq!(0b1110u32.bit_width(), 4);
    assert_eq!(u32::MAX.bit_width(), 32);
    assert_eq!(0u64.bit_width(), 0);
    assert_eq!(1u64.bit_width(), 1);
    assert_eq!(u64::MAX.bit_width(), 64);

    // Powers of two: highest and lowest one coincide
    assert_eq!(0x80u8.highest_one(), Some(7));
    assert_eq!(0x80u8.lowest_one(), Some(7));
    assert_eq!(0x8000_0000u32.isolate_highest_one(), 0x8000_0000);
    assert_eq!(0x8000_0000u32.isolate_lowest_one(), 0x8000_0000);

    // Signed extremes (bit-pattern semantics via `as unsigned`)
    assert_eq!(i32::MIN.highest_one(), Some(31));
    assert_eq!(i32::MIN.lowest_one(), Some(31));
    assert_eq!(i32::MIN.isolate_highest_one(), i32::MIN);
    assert_eq!(i32::MIN.isolate_lowest_one(), i32::MIN);
    assert_eq!((-1i32).highest_one(), Some(31));
    assert_eq!((-1i32).lowest_one(), Some(0));
    assert_eq!((-1i32).isolate_highest_one(), i32::MIN);
    assert_eq!((-1i32).isolate_lowest_one(), 1);
    assert_eq!(i32::MAX.highest_one(), Some(30));
    assert_eq!(0i32.highest_one(), None);
    assert_eq!(0i32.isolate_lowest_one(), 0);

    // NonZero doc examples
    let nz = NonZeroU32::new(0b1_0000).unwrap();
    assert_eq!(nz.highest_one(), 4);
    assert_eq!(nz.lowest_one(), 4);
    assert_eq!(nz.bit_width(), NonZeroU32::new(5).unwrap());
    assert_eq!(nz.isolate_highest_one(), nz);
    assert_eq!(nz.isolate_lowest_one(), nz);
    let nz_ones = NonZeroU32::new(0b1_1111).unwrap();
    assert_eq!(nz_ones.highest_one(), 4);
    assert_eq!(nz_ones.lowest_one(), 0);
    assert_eq!(nz_ones.isolate_highest_one().get(), 0b1_0000);
    assert_eq!(nz_ones.isolate_lowest_one().get(), 0b0_0001);
    let nz_max = NonZeroU32::new(u32::MAX).unwrap();
    assert_eq!(nz_max.bit_width().get(), 32);
    assert_eq!(nz_max.isolate_lowest_one().get(), 1);
    assert_eq!(nz_max.isolate_highest_one().get(), 0x8000_0000);
}

#[kani::proof]
fn check_u8_bit_ops() {
    let x: u8 = kani::any();

    let w = x.bit_width();
    assert!(w == 8 - x.leading_zeros());
    if x == 0 {
        assert!(w == 0);
    } else {
        assert!(w >= 1 && w <= 8);
        assert!(x.checked_shr(w).unwrap_or(0) == 0); // fits in w bits
        assert!((x >> (w - 1)) & 1 == 1); // but not in w - 1 bits
    }

    match x.highest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 8);
            assert!(i == 7 - x.leading_zeros());
            assert!((x >> i) & 1 == 1); // bit i is set
            assert!(x.checked_shr(i + 1).unwrap_or(0) == 0); // nothing above
        }
    }

    match x.lowest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 8);
            assert!(i == x.trailing_zeros());
            assert!((x >> i) & 1 == 1); // bit i is set
            assert!(x & ((1u8 << i) - 1) == 0); // nothing below
        }
    }

    let hi = x.isolate_highest_one();
    if x == 0 {
        assert!(hi == 0);
    } else {
        assert!(hi != 0 && (hi & (hi - 1)) == 0); // single bit
        assert!(x & hi == hi); // bit belongs to x
        assert!(x & !((hi << 1).wrapping_sub(1)) == 0); // no higher bits in x
        assert!(hi == 1u8 << (7 - x.leading_zeros()));
    }

    let lo = x.isolate_lowest_one();
    if x == 0 {
        assert!(lo == 0);
    } else {
        assert!(lo != 0 && (lo & (lo - 1)) == 0); // single bit
        assert!(x & lo == lo); // bit belongs to x
        assert!(x & (lo - 1) == 0); // no lower bits in x
    }
}

#[kani::proof]
fn check_u32_bit_ops() {
    let x: u32 = kani::any();

    let w = x.bit_width();
    assert!(w == 32 - x.leading_zeros());
    if x == 0 {
        assert!(w == 0);
    } else {
        assert!(w >= 1 && w <= 32);
        assert!(x.checked_shr(w).unwrap_or(0) == 0);
        assert!((x >> (w - 1)) & 1 == 1);
    }

    match x.highest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 32);
            assert!(i == 31 - x.leading_zeros());
            assert!((x >> i) & 1 == 1);
            assert!(x.checked_shr(i + 1).unwrap_or(0) == 0);
        }
    }

    match x.lowest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 32);
            assert!(i == x.trailing_zeros());
            assert!((x >> i) & 1 == 1);
            assert!(x & ((1u32 << i) - 1) == 0);
        }
    }

    let hi = x.isolate_highest_one();
    if x == 0 {
        assert!(hi == 0);
    } else {
        assert!(hi != 0 && (hi & (hi - 1)) == 0);
        assert!(x & hi == hi);
        assert!(x & !((hi << 1).wrapping_sub(1)) == 0);
        assert!(hi == 1u32 << (31 - x.leading_zeros()));
    }

    let lo = x.isolate_lowest_one();
    if x == 0 {
        assert!(lo == 0);
    } else {
        assert!(lo != 0 && (lo & (lo - 1)) == 0);
        assert!(x & lo == lo);
        assert!(x & (lo - 1) == 0);
    }
}

#[kani::proof]
fn check_u64_bit_ops() {
    let x: u64 = kani::any();

    let w = x.bit_width();
    assert!(w == 64 - x.leading_zeros());
    if x == 0 {
        assert!(w == 0);
    } else {
        assert!(w >= 1 && w <= 64);
        assert!(x.checked_shr(w).unwrap_or(0) == 0);
        assert!((x >> (w - 1)) & 1 == 1);
    }

    match x.highest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 64);
            assert!(i == 63 - x.leading_zeros());
            assert!((x >> i) & 1 == 1);
            assert!(x.checked_shr(i + 1).unwrap_or(0) == 0);
        }
    }

    match x.lowest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 64);
            assert!(i == x.trailing_zeros());
            assert!((x >> i) & 1 == 1);
            assert!(x & ((1u64 << i) - 1) == 0);
        }
    }

    let hi = x.isolate_highest_one();
    if x == 0 {
        assert!(hi == 0);
    } else {
        assert!(hi != 0 && (hi & (hi - 1)) == 0);
        assert!(x & hi == hi);
        assert!(x & !((hi << 1).wrapping_sub(1)) == 0);
        assert!(hi == 1u64 << (63 - x.leading_zeros()));
    }

    let lo = x.isolate_lowest_one();
    if x == 0 {
        assert!(lo == 0);
    } else {
        assert!(lo != 0 && (lo & (lo - 1)) == 0);
        assert!(x & lo == lo);
        assert!(x & (lo - 1) == 0);
    }
}

/// Signed methods operate on the `as u32` bit pattern (so e.g. `highest_one`
/// of any negative value is `Some(31)`).
#[kani::proof]
fn check_i32_bit_ops() {
    let x: i32 = kani::any();
    let ux = x as u32;

    match x.highest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 32);
            assert!(i == 31 - ux.leading_zeros());
            assert!((ux >> i) & 1 == 1);
            assert!(ux.checked_shr(i + 1).unwrap_or(0) == 0);
        }
    }

    match x.lowest_one() {
        None => assert!(x == 0),
        Some(i) => {
            assert!(x != 0 && i < 32);
            assert!(i == ux.trailing_zeros());
            assert!((ux >> i) & 1 == 1);
            assert!(ux & ((1u32 << i) - 1) == 0);
        }
    }

    let hi = x.isolate_highest_one() as u32;
    if x == 0 {
        assert!(hi == 0);
    } else {
        assert!(hi != 0 && (hi & (hi - 1)) == 0);
        assert!(ux & hi == hi);
        assert!(ux & !((hi << 1).wrapping_sub(1)) == 0);
        assert!(hi == 1u32 << (31 - ux.leading_zeros()));
    }

    let lo = x.isolate_lowest_one() as u32;
    if x == 0 {
        assert!(lo == 0);
    } else {
        assert!(lo != 0 && (lo & (lo - 1)) == 0);
        assert!(ux & lo == lo);
        assert!(ux & (lo - 1) == 0);
    }
}

/// The `NonZero` variants return plain indices/values (no `Option`), since
/// the input cannot be zero.
#[kani::proof]
fn check_nonzero_u32_bit_ops() {
    let nz: NonZeroU32 = kani::any();
    let x = nz.get();

    let w = nz.bit_width();
    assert!(w.get() == 32 - nz.leading_zeros());
    assert!(w.get() >= 1 && w.get() <= 32);
    assert!(x.checked_shr(w.get()).unwrap_or(0) == 0);
    assert!((x >> (w.get() - 1)) & 1 == 1);

    let hi_idx = nz.highest_one();
    assert!(hi_idx == 31 - nz.leading_zeros());
    assert!((x >> hi_idx) & 1 == 1);
    assert!(x.checked_shr(hi_idx + 1).unwrap_or(0) == 0);

    let lo_idx = nz.lowest_one();
    assert!(lo_idx == nz.trailing_zeros());
    assert!((x >> lo_idx) & 1 == 1);
    assert!(x & ((1u32 << lo_idx) - 1) == 0);

    let hi = nz.isolate_highest_one();
    assert!(hi.get() == 1u32 << hi_idx);
    assert!(x & hi.get() == hi.get());

    let lo = nz.isolate_lowest_one();
    assert!(lo.get() != 0 && (lo.get() & (lo.get() - 1)) == 0);
    assert!(x & lo.get() == lo.get());
    assert!(x & (lo.get() - 1) == 0);

    // Relation between the four: indices match the isolated bits
    assert!(lo.get() == 1u32 << lo_idx);
    assert!(hi_idx >= lo_idx);
}
