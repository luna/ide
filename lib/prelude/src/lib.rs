#![feature(trait_alias)]

pub use core::any::type_name;
pub use core::fmt::Debug;
pub use derivative::Derivative;
pub use derive_more::*;
pub use failure::Fail;
pub use itertools::Itertools;
pub use shrinkwraprs::Shrinkwrap;
pub use std::cell::Ref;
pub use std::cell::RefCell;
pub use std::collections::HashMap;
pub use std::fmt::Display;
pub use std::ops::Deref;
pub use std::ops::DerefMut;
pub use std::rc::Rc;
pub use std::rc::Weak;
pub use std::iter;
pub use std::iter::FromIterator;
pub use std::marker::PhantomData;
pub use num::Num;
pub use std::convert::identity;
pub use std::ops::Index;
pub use std::ops::IndexMut;
pub use std::slice::SliceIndex;

pub trait Str = AsRef<str>;

pub fn default<T: Default>() -> T {
    Default::default()
}
