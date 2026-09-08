// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Public qualification corpus for the core-v2 profile.
//!
//! Generic, dependency-free proof harnesses that exercise the qualified
//! toolchain end to end: arithmetic and bounded loops with unwinding
//! assertions, assumptions and covers, derived `kani::Arbitrary`, a small
//! selection of Rust 1.96/1.97.1-stabilized library items (mirroring the
//! compatibility corpus in `tests/kani/Compat196` and `tests/kani/Compat197`,
//! portable values only), and at least one expected-unreachable check.
//!
//! All harnesses must verify with default `cargo kani` flags; loop bounds
//! are given in-source via `#[kani::unwind]`.

#[kani::proof]
fn check_u32_add_commutes_bounded() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    kani::assume(x < 1000 && y < 1000);
    assert!(x + y == y + x);
}

#[kani::proof]
#[kani::unwind(9)]
fn check_bounded_loop_sum() {
    let n: u8 = kani::any();
    kani::assume(n <= 8);
    let mut sum: u32 = 0;
    let mut i: u32 = 1;
    while i <= n as u32 {
        sum += i;
        i += 1;
    }
    assert!(2 * sum == (n as u32) * (n as u32 + 1));
}

/// Assumptions prune the failing branch, so its assertion is reported as
/// unreachable; the cover is satisfiable on the remaining paths.
#[kani::proof]
fn check_assume_prunes_branch() {
    let x: u8 = kani::any();
    kani::assume(x < 10);
    if x >= 10 {
        assert!(x > 200);
    }
    kani::cover!(x > 5);
}

#[kani::proof]
fn check_sort3_network() {
    let a: u8 = kani::any();
    let b: u8 = kani::any();
    let c: u8 = kani::any();
    let (mut x, mut y, mut z) = (a, b, c);
    if x > y {
        core::mem::swap(&mut x, &mut y);
    }
    if y > z {
        core::mem::swap(&mut y, &mut z);
    }
    if x > y {
        core::mem::swap(&mut x, &mut y);
    }
    assert!(x <= y && y <= z);
    assert!(x == a.min(b).min(c));
    assert!(z == a.max(b).max(c));
}

#[cfg_attr(kani, derive(kani::Arbitrary))]
#[derive(Clone, Copy)]
struct Point {
    x: i8,
    y: i8,
}

fn manhattan(p: Point, q: Point) -> i32 {
    (p.x as i32 - q.x as i32).abs() + (p.y as i32 - q.y as i32).abs()
}

#[kani::proof]
fn check_manhattan_metric() {
    let p: Point = kani::any();
    let q: Point = kani::any();
    assert!(manhattan(p, q) == manhattan(q, p));
    assert!(manhattan(p, p) == 0);
    kani::cover!(manhattan(p, q) > 300);
}

/// Rust 1.97.0 stabilized integer bit inspection, symbolic vs relational
/// specs (mirrors `tests/kani/Compat197/bit_ops.rs`): `highest_one` and
/// `lowest_one` return the bit index as an `Option` (`None` at 0);
/// `isolate_*` keep the bit value; `bit_width` is `BITS - leading_zeros`.
#[kani::proof]
fn check_u32_bit_ops_197() {
    let x: u32 = kani::any();
    match x.highest_one() {
        None => assert!(x == 0),
        Some(i) => assert!(x != 0 && i == 31 - x.leading_zeros()),
    }
    let nz: core::num::NonZeroU32 = kani::any();
    let v = nz.get();
    assert!(nz.bit_width().get() == 32 - nz.leading_zeros());
    let hi_idx = nz.highest_one();
    assert!(hi_idx == 31 - nz.leading_zeros());
    assert!((v >> hi_idx) & 1 == 1);
    let lo_idx = nz.lowest_one();
    assert!(lo_idx == nz.trailing_zeros());
    assert!(nz.isolate_highest_one().get() == 1u32 << hi_idx);
    assert!(nz.isolate_lowest_one().get() == (v & v.wrapping_neg()));
    assert!(hi_idx >= lo_idx);
}

/// Rust 1.96.0 `core::range` types: `Copy`, `IntoIterator`, legacy
/// conversion (mirrors `tests/kani/Compat196/core_range.rs`).
#[kani::proof]
#[kani::unwind(6)]
fn check_core_range_iter_196() {
    let n: u8 = kani::any();
    kani::assume(n <= 4);
    let r = core::range::Range::from(0u8..n);
    let r_copy = r;
    let mut count: u8 = 0;
    for _ in r_copy {
        count += 1;
    }
    assert!(count == n);
    assert!(r.start == 0 && r.end == n);
}

/// Rust 1.96.0 `From<T> for LazyCell` initialized fast path; the explicit
/// type annotation is required (E0283 otherwise).
#[kani::proof]
fn check_lazy_cell_from_196() {
    let cell: core::cell::LazyCell<u32> = core::cell::LazyCell::from(41u32);
    assert!(*cell == 41);
}

/// Rust 1.97.0 `cfg(target_has_atomic_primitive_alignment)`: the "ptr"
/// value holds on both qualified targets (aarch64-apple-darwin and
/// x86_64-unknown-linux-gnu); the target-differential "128" value is
/// deliberately not used here (mirrors
/// `tests/kani/Compat197/atomic_primitive_alignment.rs`).
#[kani::proof]
fn check_atomic_alignment_ptr_197() {
    assert!(cfg!(target_has_atomic_primitive_alignment = "ptr"));
    assert_eq!(
        core::mem::align_of::<core::sync::atomic::AtomicUsize>(),
        core::mem::align_of::<usize>()
    );
}

/// Rust 1.97.0 `Default for core::iter::RepeatN`: the default is the empty
/// iterator, equivalent to `repeat_n(v, 0)`.
#[kani::proof]
fn check_repeat_n_default_197() {
    let empty: core::iter::RepeatN<u8> = Default::default();
    assert!(empty.count() == 0);
    assert!(core::iter::repeat_n(7u8, 0).count() == 0);
}

/// Rust 1.97.0 const-stabilized `char::is_control`: symbolic char against
/// the C0/C1 reference ranges.
#[kani::proof]
fn check_char_is_control_197() {
    const NL_IS_CONTROL: bool = '\n'.is_control();
    assert!(NL_IS_CONTROL);
    let c: char = kani::any();
    let reference = (c <= '\u{1F}') || ('\u{7F}'..='\u{9F}').contains(&c);
    assert!(c.is_control() == reference);
}

#[kani::proof]
fn check_midpoint_widening() {
    let a: u8 = kani::any();
    let b: u8 = kani::any();
    let m = ((a as u16 + b as u16) / 2) as u8;
    assert!(m >= a.min(b));
    assert!(m <= a.max(b));
    kani::cover!(a > 200 && b < 50);
}
