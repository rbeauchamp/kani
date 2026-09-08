// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.97.0 compatibility corpus: `Default for std::iter::RepeatN`.
//! The default is an empty iterator (`RepeatN { inner: None }` in std) --
//! like `repeat_n(value, 0)` but without needing a value.

/// The default `RepeatN` yields zero elements (bounded loop form).
#[kani::proof]
#[kani::unwind(4)]
fn check_repeat_n_default_loop() {
    let r: std::iter::RepeatN<u64> = Default::default();
    let mut count = 0u32;
    for _ in r {
        count += 1;
    }
    assert!(count == 0);
}

/// Emptiness is also visible through the iterator API: zero length, no
/// first element, and it stays exhausted.
#[kani::proof]
fn check_repeat_n_default_is_empty() {
    let mut r: std::iter::RepeatN<u32> = Default::default();
    assert_eq!(r.size_hint(), (0, Some(0)));
    assert_eq!(r.len(), 0);
    assert_eq!(r.next(), None);
    assert_eq!(r.next(), None); // stays exhausted
    assert_eq!(r.count(), 0);
}

/// For any element, the default behaves exactly like `repeat_n(v, 0)`.
#[kani::proof]
fn check_repeat_n_default_equals_repeat_n_zero() {
    let v: u16 = kani::any();
    let a: std::iter::RepeatN<u16> = Default::default();
    let b = std::iter::repeat_n(v, 0);
    assert!(a.eq(b));
}

/// Control: a non-empty `RepeatN` does yield its element, so the emptiness
/// assertions above are not vacuous.
#[kani::proof]
fn check_repeat_n_contrast_nonzero_count() {
    let v: u8 = kani::any();
    let mut r = std::iter::repeat_n(v, 2);
    assert_eq!(r.next(), Some(v));
    assert_eq!(r.next(), Some(v));
    assert_eq!(r.next(), None);
}
