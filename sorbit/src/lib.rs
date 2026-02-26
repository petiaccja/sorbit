#![warn(missing_docs)]

//! # Sorbit
//!
//! Sorbit is a framework for serializing and deserializing data structures in
//! an exact binary layout.
//!
//! ## Serializing data structures
//!
//! Like other serialization frameworks, sorbit comes with the derive macros
//! [`Serialize`] and [`Deserialize`] to implement the [`serialize::Serialize`]
//! and [`deserialize::Deserialize`] traits, but you can also manually implement
//! these traits. Once implemented for your data structures, you can serialize
//! them using a [`serialize::Serializer`] and deserialize them using a
//! [`deserialize::Deserializer`].
//!
//! Example:
//! ```
//! use sorbit::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! #[sorbit(byte_order=big_endian)]
//! struct Data {
//!     field: u32,
//! }
//! ```
//!
//! While you can implement the serializer
//! traits yourself, sorbit already comes with the [`serialize::StreamSerializer`]
//! and [`deserialize::StreamDeserializer`] objects. These provide an abstraction
//! over streams (like files or in-memory buffers). For general use, sorbit
//! gives you the [`io::GrowingMemoryStream`], and for use in `no_std` environments,
//! you have the [`io::FixedMemoryStream`].
//!
//! Example:
//! ```
//! # use sorbit::{Serialize, Deserialize};
//! #
//! # #[derive(Serialize, Deserialize)]
//! # #[sorbit(byte_order=big_endian)]
//! # struct Data {
//! #     field: u32,
//! # }
//! #
//! use sorbit::io::GrowingMemoryStream;
//! use sorbit::serialize::StreamSerializer;
//! use sorbit::serialize::Serialize as _;
//!
//! let data = Data{ field: 345 };
//! let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
//! data.serialize(&mut serializer);
//! ```
//!
//! ## Specifying layouts
//!
//! In sorbit, there are four types of entities:
//! - `struct`s
//!   - fields: any field of the struct that is not annotated with a
//!     `sorbit(bit_field=<NAME>)` attribute.
//!   - bit fields: there is a virtual field that is only present in the serialized
//!     data structure. It's declared via the `sorbit(bit_field=<NAME>, repr=<TYPE>)`
//!     attribute that is present on any field of a `struct`. Once declared, fields
//!     of the struct can be assigned to this bit field by specifying its name in
//!     the field's annotation, like `sorbit(bit_field=<NAME>, bits=<N>)`. As the
//!     fields that belong to the same bit field must come consecutively in the
//!     original `struct`, you can think of them as being merged into a bit field
//!     with the given `<NAME>`.
//! - `enum`s
//!
//! The `sorbit` attribute has the following options:
//! - `bit_field=<NAME>`: specifies the bit field the structure's field belongs to.
//! - `repr=<TYPE>`: specifies the bit field's representation. Typically an integer,
//!   but it can be any type. See the [`bit::PackInto`] and [`bit::UnpackFrom`] traits.
//! - `bits=<N>`, `bits=<N>..<M>`, or `bits=<N>..=<M>`: the bits the field will occupy
//!   in its containing bit field.
//! - `offset=<N>`: the absolute byte offset of the field within the struct.
//!   Zero padding is applied to reach the required offset.
//! - `align=<N>`: the alignment in bytes of the field within the struct.
//!   Zero padding is applied to reach the required alignment.
//! - `round=<N>`: the object's size is rounded up to a multiple of this value in bytes.
//!   Zero padding is applied to achieve the rounded size. This is applicable to both
//!   fields and entire structs.
//! - `len=<N>`: specifies the total length of a struct. Zero padding is applied.
//! - `byte_order=big_endian|little_endian`: specifies the endianness of a field
//!   bit field, or struct.
//! - `bit_numbering=LSB0|MSB0`: specifies the bit numbering within a bit field.
//!   This does not change the data written to disk, like endianness, it merely
//!   changes how you specify the bit field's members. You should set this to
//!   be equivalent to the third party specification's conventions for which
//!   you're implementing the data structures.
//!
//! Using the above options, you can precisely describe the serialized binary
//! format. Take, for example, this data structure:
//!
//! ```plaintext
//! | MSB | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 | LSB
//! |  0  | flag  |       command         |
//! |  1  |            reserved           |
//! |  2  | MSB         param             >
//! |  3  >                           LSB |
//! ```
//!
//! This data structure consists of four bytes in total. The first byte is a bit
//! field, its first member `flag` occupying two bits, and its second member
//! `command` occupying 6 bits. This is followed by one byte of padding, which
//! is set to zero. There is an additional field `param` in the
//! last two bytes of the structure. Looking at the annotations, we can tell
//! that the structure is using big endian byte order LSB0 bit numbering.
//!
//! With `sorbit`, we could write:
//!
//! ```
//! # use sorbit::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! #[sorbit(byte_order=big_endian)]
//! struct message {
//!     #[sorbit(bit_field=_byte0, repr=u8, bit_numbering=LSB0)]
//!     #[sorbit(bits=6..=7)]
//!     flag: u8,
//!     #[sorbit(bit_field=_byte0, bits=0..=5)]
//!     command: u8,
//!     
//!     #[sorbit(offset=2)]
//!     param: u16,
//! }
//! ```
//!
//! ## Implementing custom serializers
//!
//! You can implement your own serializers via manually implementing the
//! [`serialize::Serializer`] and [`deserialize::Deserializer`] traits.
//! There is little reason to this, because the [`serialize::StreamSerializer`]
//! and [`deserialize::StreamDeserializer`] implementations can serialize into
//! any binary stream, so they cover almost all the use cases. Instead, you
//! would typically implement the [`io::Read`], [`io::Write`], and [`io::Seek`]
//! traits for your stream, and then you can use them through the aforementioned
//! serializers.
//!
//! Implementing the serializer traits can be useful if you don't want to stick
//! to binary layouts, but you need something else, like text. While this
//! scenario is supported by sorbit, such serializers won't respect sorbit's
//! binary layout attributes, and sorbit provides no way to specify general
//! purpose attributes. Unless you rely on sorbit as your primary serializer
//! and you only need a quick and simple solution without pulling in another
//! serialization framework, go ahead. However, in most cases, you're better
//! off using a general purpose framework like [serde](https://docs.rs/serde/latest/serde/).
//!
//! ## `no_std` and embedded
//!
//! Sorbit was designed with `no_std` as a first-class feature: all the core
//! features work without `std` and `alloc`. When `std` and `alloc` are enabled,
//! you get access to additional features such as the [`io::GrowingMemoryStream`].
//! When allocations are not acceptable, you can rely on the no-alloc features
//! of sorbit like the [`io::FixedMemoryStream`].

// Disable the [`std`] standard crate when the "std" feature is not enabled.
#![cfg_attr(not(feature = "std"), no_std)]

// Enable the [`alloc`] standard crate when the "alloc" feature is enabled.
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bit;
pub mod byte_order;
pub mod deserialize;
pub mod error;
pub mod io;
pub mod serialize;
pub use sorbit_derive::{Deserialize, Serialize};

mod types;

extern crate self as sorbit;
