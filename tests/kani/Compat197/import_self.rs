// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rust 1.97.0 compatibility corpus: the import grammar accepts `self`
//! with a rename in use declarations, both for modules
//! (`use some_mod::self as alias;`) and for types
//! (`use some_mod::SomeEnum::self as Alias;`). Both forms compile on
//! stable 1.97.1 (edition 2015) and the aliased paths are fully usable.

mod some_mod {
    pub fn seven() -> u32 {
        7
    }

    #[derive(Debug, PartialEq)]
    pub enum SomeEnum {
        X,
        Y,
    }
}

use some_mod::SomeEnum::self as Alias;
use some_mod::self as alias;

#[kani::proof]
fn check_import_self_aliases() {
    // module alias path
    assert_eq!(alias::seven(), 7);

    // enum alias path
    let e = Alias::X;
    assert!(e == some_mod::SomeEnum::X);
    assert!(e != Alias::Y);
}
