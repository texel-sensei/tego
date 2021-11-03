//! Module to contain some simple math primitives for working with pixel coordinates.

use std::ops;

macro_rules! define_vector {
    ($name:ident $T:ty) => {
        #[derive(Debug, PartialEq, Copy, Clone, Default)]
        #[allow(non_camel_case_types)]
        pub struct $name {
            pub x: $T,
            pub y: $T,
        }

        impl $name {
            pub fn new(x: $T, y: $T) -> Self { Self { x, y } }
        }

        impl_op_ex!{+ |a: &$name, b: &$name| -> $name { $name::new(a.x + b.x, a.y + b.y )}}
        impl_op_ex!{- |a: &$name, b: &$name| -> $name { $name::new(a.x - b.x, a.y - b.y )}}
        impl_op_ex!{* |a: &$name, b: &$name| -> $name { $name::new(a.x * b.x, a.y * b.y )}}
        impl_op_ex!{/ |a: &$name, b: &$name| -> $name { $name::new(a.x / b.x, a.y / b.y )}}
    };
}

// Define vector of integers
define_vector!{ivec2 i32}
impl_op_ex_commutative!{* |a: &ivec2, f: &i32| -> ivec2 { ivec2::new(a.x * f, a.y * f )}}
impl Eq for ivec2 {}

// Define vector of floats
define_vector!{fvec2 f32}
impl_op_ex_commutative!{* |a: &fvec2, f: &f32| -> fvec2 { fvec2::new(a.x * f, a.y * f )}}


/// Struct that defines a rectangle given by its upper left corner and extends.
#[derive(Debug, PartialEq, Eq)]
pub struct Rect {
    pub upper_left: ivec2,
    pub size: ivec2,
}

impl Rect {
    pub fn new(upper_left: ivec2, size: ivec2) -> Self { Self { upper_left, size } }
}
