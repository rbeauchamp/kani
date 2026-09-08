// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.96.0 compatibility corpus: layout of `#[repr(Int)]` enums that
//! have variants carrying uninhabited ZST fields. Since the 1.96 fix, such
//! a variant contributes nothing to the enum's size or alignment.
//!
//! The uninhabited ZST here is an empty enum (`Void`), which is
//! constructible on stable 1.97.1. Using the never type directly
//! (`B(!)`) is still feature-gated (`error[E0658]: the '!' type is
//! experimental`), so it cannot appear in a stable-toolchain corpus.
//!
//! Expected layouts below were pinned with rustc 1.97.1
//! (aarch64-apple-darwin).

extern crate core;

#[allow(dead_code)]
enum Void {}

#[allow(dead_code)]
#[repr(u8)]
enum Small {
    A,
    B(Void),
}

#[allow(dead_code)]
#[repr(u16)]
enum Wide {
    A(u8),
    B(Void),
}

#[allow(dead_code)]
#[repr(i8)]
enum Signed {
    A,
    B(i8),
    C(Void),
}

#[allow(dead_code)]
#[repr(u8)]
enum Pair {
    A(u8, u8),
    B(Void),
}

#[kani::proof]
fn check_repr_int_layout_with_uninhabited_fields() {
    assert_eq!(core::mem::size_of::<Void>(), 0);
    assert_eq!(core::mem::align_of::<Void>(), 1);

    // only the tag: the uninhabited field adds nothing
    assert_eq!(core::mem::size_of::<Small>(), 1);
    assert_eq!(core::mem::align_of::<Small>(), 1);

    // tag(u16) + payload(u8) + tail padding
    assert_eq!(core::mem::size_of::<Wide>(), 4);
    assert_eq!(core::mem::align_of::<Wide>(), 2);

    // tag(i8) + payload(i8)
    assert_eq!(core::mem::size_of::<Signed>(), 2);
    assert_eq!(core::mem::align_of::<Signed>(), 1);

    // tag(u8) + two payload bytes
    assert_eq!(core::mem::size_of::<Pair>(), 3);
    assert_eq!(core::mem::align_of::<Pair>(), 1);
}

/// The same sizes are observable at runtime through `size_of_val` on the
/// inhabited variants.
#[kani::proof]
fn check_repr_int_size_of_val() {
    let s = Small::A;
    assert_eq!(core::mem::size_of_val(&s), 1);

    let w = Wide::A(200);
    assert_eq!(core::mem::size_of_val(&w), 4);

    let g = Signed::B(-1);
    assert_eq!(core::mem::size_of_val(&g), 2);

    let p = Pair::A(1, 2);
    assert_eq!(core::mem::size_of_val(&p), 3);
}
