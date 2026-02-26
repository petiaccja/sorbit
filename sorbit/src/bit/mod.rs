//! This module contains tooling to construct bit fields at runtime during serialization.
//!
//! The [`BitField`] stores the bit field and lets you *pack* and *unpack*
//! its members.
//!
//! When a member is *packed*, it is first compressed into an arbitrary
//! width representation, and then the arbitrary representation bits are copied
//! into the bit field at requested bit offset.
//! For example, a member may be represented as a `i8` while you're
//! working with it. During packing, it is converted to a 5-bit two's complement
//! representation, and those 5 bits are copied into the bit field. For a member
//! to be packed into the bit field, its type must implement [`PackInto`]. The
//! trait is implemented for primitive integers out of the box, but you can
//! also implement it yourself.
//!
//! *Unpacking* takes the same steps, but in reverse. The 5 bits in two's complement
//! are extracted from the bit field, then they are expanded to an `i8`, from
//! which point you can work with it as a normal Rust type. Similary to packing,
//! types must implement [`UnpackFrom`].

mod bit_field;
mod bit_pack;
mod bit_util;
mod error;

pub use bit_field::BitField;
pub use bit_pack::{PackInto, UnpackFrom};
pub use bit_util::{bit_size_of, bit_size_of_val};
pub use error::Error;
