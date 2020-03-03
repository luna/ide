//! Code dealing with executors, i.e. entities that are used to execute asynchronous
//! computations, like `Future`s or `Stream`s.

pub mod web;
pub mod global;

#[cfg(test)]
pub mod test_utils;