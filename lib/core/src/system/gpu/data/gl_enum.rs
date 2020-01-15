//! This module defines a wrapper for WebGL enums and associated utils.

use crate::prelude::*;
use crate::system::gpu::shader::Context;



// ==============
// === GlEnum ===
// ==============

newtype_copy! {
    /// The newtype for WebGL enums.
    GlEnum(u32);
}



// ==================
// === Extensions ===
// ==================

/// Extension methods.
pub mod traits {
    use super::*;

    /// Methods for every object which implements `Into<GlEnum>`.
    pub trait IntoGlEnum {
        /// Converts the current value to `GlEnum`.
        fn into_gl_enum(&self) -> GlEnum;
    }

    impl<T> IntoGlEnum for T where for<'a> &'a T:Into<GlEnum> {
        fn into_gl_enum(&self) -> GlEnum {
            self.into()
        }
    }

    /// Methods for every object which implements `PhantomInto<GlEnum>`.
    pub trait PhantomIntoGlEnum {
        /// Converts the current value to `GlEnum`.
        fn gl_enum() -> GlEnum;
    }

    impl<T> PhantomIntoGlEnum for T where T:PhantomInto<GlEnum> {
        fn gl_enum() -> GlEnum {
            T::phantom_into::<GlEnum>()
        }
    }
}



// ==============
// === Macros ===
// ==============

/// Combination of `define_singletons` and `define_gl_enum_conversions`.
#[macro_export]
macro_rules! define_singletons_gl {
    ( $( $(#$meta:tt)* $name:ident = $expr:expr ),* $(,)? ) => {
        shapely::define_singletons!{ $( $(#$meta)* $name),* }
        $crate::define_gl_enum_conversions!{ $( $(#$meta)* $name = $expr ),* }
    }
}


/// Defines conversions `From<$type>` and `From<PhantomData<$type>>` for every provided type.
#[macro_export]
macro_rules! define_gl_enum_conversions {
    ( $( $(#$meta:tt)* $type:ty = $expr:expr ),* $(,)? ) => {
        $(
            impl From<$type> for GlEnum {
                fn from(_:$type) -> Self {
                    $expr.into()
                }
            }

            impl From<PhantomData<$type>> for GlEnum {
                fn from(_:PhantomData<$type>) -> Self {
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


// ================================
// === Primitive Type Instances ===
// ================================

define_gl_enum_conversions! {
    bool = Context::BOOL,
    i32  = Context::INT,
    f32  = Context::FLOAT,
}
