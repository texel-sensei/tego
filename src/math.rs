//! Module to contain some simple math primitives for working with pixel coordinates.

use std::ops;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct ivec2 {
    pub x: i32,
    pub y: i32,
}

impl ivec2 {
    pub fn new(x: i32, y: i32) -> Self { Self { x, y } }
}

impl_op_ex!{+ |a: &ivec2, b: &ivec2| -> ivec2 { ivec2::new(a.x + b.x, a.y + b.y )}}
impl_op_ex!{- |a: &ivec2, b: &ivec2| -> ivec2 { ivec2::new(a.x - b.x, a.y - b.y )}}
impl_op_ex!{* |a: &ivec2, b: &ivec2| -> ivec2 { ivec2::new(a.x * b.x, a.y * b.y )}}
impl_op_ex!{/ |a: &ivec2, b: &ivec2| -> ivec2 { ivec2::new(a.x / b.x, a.y / b.y )}}
impl_op_ex_commutative!{* |a: &ivec2, f: &i32| -> ivec2 { ivec2::new(a.x * f, a.y * f )}}

/// Struct that defines a rectangle given by its upper left corner and extends.
#[derive(Debug, PartialEq, Eq)]
pub struct Rect {
    pub upper_left: ivec2,
    pub size: ivec2,
}

impl Rect {
    pub fn new(upper_left: ivec2, size: ivec2) -> Self { Self { upper_left, size } }
}
