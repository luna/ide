//! This module defines a wrapper for WebGL enums and associated utils.

// ==============
// === GlEnum ===
// ==============

/// The newtype for WebGL enums.
#[derive(Clone,Copy,Debug)]
pub struct GlEnum {
    /// Raw value of the enum.
    pub raw: u32,
}

impl From<u32> for GlEnum {
    fn from(raw:u32) -> Self {
        Self {raw}
    }
}

impl From<GlEnum> for u32 {
    fn from(t:GlEnum) -> Self {
        t.raw
    }
}



// =================
// === GlEnumOps ===
// =================

/// Methods for every object which implements `Into<GlEnum>`.
pub trait GlEnumOps {
    /// Converts the current value to `GlEnum`.
    fn to_gl_enum<G:From<GlEnum>>(&self) -> G;
}

impl<T> GlEnumOps for T where for<'a> &'a T:Into<GlEnum> {
    fn to_gl_enum<G:From<GlEnum>>(&self) -> G {
        let g:GlEnum = self.into();
        g.into()
    }
}



// ==============
// === Macros ===
// ==============

/// Defines singleton types, just like `define_singletons`. Then it also defines conversions
/// `From<$singleton>` and `From<PhantomData<$singleton>>` for every singleton type.
#[macro_export]
macro_rules! define_singletons_gl {
    ( $( $(#$meta:tt)* $name:ident = $expr:expr ),* $(,)? ) => {
        shapely::define_singletons!{ $( $(#$meta)* $name),* }
        $(
            impl From<$name> for GlEnum {
                fn from(_:$name) -> Self {
                    $expr.into()
                }
            }

            impl From<PhantomData<$name>> for GlEnum {
                fn from(_:PhantomData<$name>) -> Self {
                    $expr.into()
                }
            }
        )*
    }
}

/// Defines singletons and an associated enum type, just like `define_singleton_enum`.
/// It also defines conversions `From<$singleton>` and `From<PhantomData<$singleton>>` for every
/// singleton type and for the whole enum type.
#[macro_export]
macro_rules! define_singleton_enum_gl {
    (
        $(#$meta:tt)*
        $name:ident {
            $( $(#$field_meta:tt)* $field:ident = $expr:expr),* $(,)?
        }
    ) => {
        $crate  :: define_singletons_gl!       { $($(#$field_meta)* $field = $expr),* }
        shapely :: define_singleton_enum_from! { $(#$meta)* $name {$($(#$field_meta)* $field),*}}

        impl From<&$name> for GlEnum {
            fn from(t:&$name) -> Self {
                match t {
                    $($name::$field => $field.into()),*
                }
            }
        }
    }
}
