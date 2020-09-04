//! Span tree shape and contained data depends not only on the AST but also some context-dependent
//! information. This module defined trait [`Context`] that provides the information known to
//! Span Tree during its construction.

use crate::prelude::*;

use crate::ParameterInfo;
use ast::Id;


/// Additional information available on nodes that are an invocation of a known methods.
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct InvocationInfo {
    /// Information about arguments taken by a called method.
    pub parameters : Vec<ParameterInfo>,
}



// ===============
// === Context ===
// ===============

/// Entity that is able to provide information whether a given expression is a known method
/// invocation. If so, additional information is provided.
pub trait Context {
    /// Checks if the given expression is known to be a call to a known method. If so, returns the
    /// available information.
    fn invocation_info(&self, id:Id, name:Option<&str>) -> Option<InvocationInfo>;

    /// Build a new context that merges this context and the one given in argument that will be used
    /// as a fallback.
    fn merge<U>(self, other:U) -> Merged<Self,U>
        where Self:Sized, U:Context {
        Merged::new(self,other)
    }
}



// ===============
// === Context ===
// ===============

/// Represents a context created from merging two other contexts.
#[derive(Clone,Debug)]
pub struct Merged<First,Second> {
    first  : First,
    second : Second
}

impl<First,Second> Merged<First,Second> {
    /// Creates a context merging the contexts from arguments.
    ///
    /// The first context is checked first, the second one is used as a fallback.
    pub fn new(first:First, second:Second) -> Self {
        Self {
            first,second
        }
    }
}

impl<First,Second> Context for Merged<First,Second>
    where First  : Context,
          Second : Context {
    fn invocation_info(&self, id: Id, name:Option<&str>) -> Option<InvocationInfo> {
        self.first.invocation_info(id, name).or_else(||
            self.second.invocation_info(id, name))
    }
}



// =============
// === Empty ===
// =============

/// An empty context that provides no information whatsoever.
#[derive(Copy,Clone,Debug)]
pub struct Empty;

impl Context for Empty {
    fn invocation_info(&self, _id:Id, _name:Option<&str>) -> Option<InvocationInfo> {
        None
    }
}

