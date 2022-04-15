#![cfg_attr(feature = "test", feature(test))]

extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod error;
mod hash;
pub mod note;
pub mod utils;
pub mod vault;

pub use hash::{HashMap, IdMap};
#[doc(inline)]
pub use note::{Note, NoteId};
#[doc(inline)]
pub use vault::{Vault, VaultBuilder};
