pub mod physics;
pub mod animator;
pub mod position;
pub mod easing;



// =============
// === Utils ===
// =============

use std::ops::Mul;
use std::ops::Add;

/// A generic trait constraint for interpolable types.
pub trait Interpolable<T:Copy> = Mul<f32, Output = T> + Add<T, Output = T> + Copy;

/// Linear interpolation function for any type implementing T * f32 and T + T.
pub fn linear_interpolation<T:Interpolable<T>>(a:T, b:T, t:f32) -> T {
    a * (1.0 - t) + b * t
}
