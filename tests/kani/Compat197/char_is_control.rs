// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.97.0 compatibility corpus: `char::is_control` is const-stable
//! since 1.97.0. The std definition (core/src/char/methods.rs) is the
//! hard-coded Unicode `Cc` set:
//!   matches!(self, '\0'..='\x1f' | '\x7f'..='\u{9f}')

// Const-context uses, evaluated at compile time.
const A_IS_CONTROL: bool = 'a'.is_control();
const NUL_IS_CONTROL: bool = '\0'.is_control();
const US_IS_CONTROL: bool = '\u{1f}'.is_control();
const SPACE_IS_CONTROL: bool = ' '.is_control();
const DEL_IS_CONTROL: bool = '\u{7f}'.is_control();
const NEL_IS_CONTROL: bool = '\u{85}'.is_control();
const PAD_IS_CONTROL: bool = '\u{9f}'.is_control();
const NBSP_IS_CONTROL: bool = '\u{a0}'.is_control();
const MAX_IS_CONTROL: bool = char::MAX.is_control();

#[kani::proof]
fn check_is_control_const() {
    assert!(!A_IS_CONTROL);
    assert!(NUL_IS_CONTROL);
    assert!(US_IS_CONTROL); // U+001F is the last C0 control
    assert!(!SPACE_IS_CONTROL); // U+0020 is not a control
    assert!(DEL_IS_CONTROL); // U+007F
    assert!(NEL_IS_CONTROL); // U+0085, a C1 control
    assert!(PAD_IS_CONTROL); // U+009F is the last C1 control
    assert!(!NBSP_IS_CONTROL); // U+00A0 is Zs, not Cc
    assert!(!MAX_IS_CONTROL);
}

/// Symbolic char: `is_control` agrees with the reference range expression
/// on every scalar value.
#[kani::proof]
fn check_is_control_symbolic() {
    let c: char = kani::any();
    let reference = (c as u32) < 0x20 || (0x7f..=0x9f).contains(&(c as u32));
    assert!(c.is_control() == reference);
}
