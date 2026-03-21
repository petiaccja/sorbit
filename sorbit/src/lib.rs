#![warn(missing_docs)]

//! # Sorbit
//!
//! *Notice: the API is not yet stable.*
//!
//! Sorbit is a framework for serializing and deserializing data structures in
//! an exact binary layout. Sorbit works like other serialization frameworks,
//! but it gives you control over details like endianness, bit numbering,
//! arbitrary width types (i.e. bit fields), alignment, etc.
//!
//! The binary layout is typically dictated by some standard or specification,
//! usually for implementing a communication protocol. This includes, for example
//! network protocols (IP, TCP, UDP, etc.) and hardware commands (ATA, SCSI).
//! Using sorbit, you can use a regular Rust data structure, and turn it into
//! the exact bits you want using sorbit's attributes and derive capabilities.
//!
//! ## Example
//!
//! With sorbit, the Inquiry data structure from the SCSI command set could look like this:
//!
//! ```
//! use sorbit::{Deserialize, Serialize};
//! use sorbit::io::FixedMemoryStream;
//! use sorbit::ser_de::{Serialize, Serializer};
//! use sorbit::stream_ser_de::StreamSerializer;
//!
//! #[derive(Serialize, Deserialize)]
//! #[sorbit(byte_order=big_endian)]
//! struct Inquiry {
//!     #[sorbit(bit_field=_byte0, repr=u8, bit_numbering=LSB0)]
//!     #[sorbit(bits=5..=7)]
//!     peripheral_qualifier: u8,
//!     #[sorbit(bit_field=_byte0, bits=0..=4)]
//!     peripheral_device_type: u8,
//!
//!     #[sorbit(bit_field=_byte1, repr=u8, bit_numbering=LSB0, bits=7)]
//!     rmb: bool,
//!
//!     version: u8,
//!     
//!     // Subsequent fields omitted for simplicity.
//! }
//!
//! type Error = <StreamSerializer<FixedMemoryStream<[u8; 36]>> as Serializer>::Error;
//!
//! fn to_bytes(inquiry: &Inquiry) -> Result<Vec<u8>, Error> {
//!     let mut buffer = [0u8; 36];
//!     let stream = FixedMemoryStream::new(&mut buffer);
//!     let mut serializer = StreamSerializer::new(stream);
//!     inquiry.serialize(&mut serializer)?;
//!     Ok(buffer.into())
//! }
//!
//! fn to_bytes_simpler(inquiry: &Inquiry) -> Result<Vec<u8>, Error> {
//!     use sorbit::ser_de::ToBytes;
//!     inquiry.to_bytes()
//! }
//! ```
//!
//! ## Design
//!
//! ### Serializable objects
//!
//! Any type can implement the [ser_de::Serialize] and [ser_de::Deserialize]
//! traits, which make the object serializable and deserializable.
//!
//! There are two ways to implement these traits:
//! - Using the [`Serialize`] and [`Deserialize`] derive macros. With this
//!   approach, you can leverage the `#[sorbit(...)]` attributes to control the
//!   binary layout of the data. The derive macros cover most of your needs, and
//!   you should use them for simplicity and robustness whenever possible.
//! - Manually deriving [ser_de::Serialize] and [ser_de::Deserialize].
//!   The layout control attributes cannot express everything, and in those
//!   cases you need to derive the traits by hand. Use this approach only
//!   when necessary.
//!
//! ### Serializers
//!
//! Objects can be serialized into a [ser_de::Serializer], and deserialized from a
//! [ser_de::Deserializer]. The serializers and deserializers own the serialized
//! bytes, typically in the form of a stream (e.g. TCP stream).
//!
//! Sorbit ships with a generic [stream_ser_de::StreamSerializer] and a
//! [stream_ser_de::StreamDeserializer]. These serializers can serialize data
//! into any byte stream. While it's possible to implement your own serializers,
//! the stream serializers will likely cover most of your needs, and all you
//! need to do is implement some streams.
//!
//! #### Non-binary serializers
//!
//! It's possible to implement serializers in sorbit that aren't aimed at
//! binary layouts. These could be either text-based or using some special
//! binary encoding. In such a serializer, the padding and alignment functions
//! would likely be a no-op. While there are valid use cases for such an approach,
//! sorbit isn't a general serialization framework, and you'll find the features
//! lacking as soon as you want anything more advanced in such formats.
//!
//! ### Streams
//!
//! Streams implement a subset of the [`io::Read`], [`io::Write`], and [`io::Seek`]
//! traits. These are analogous to the `std` equivalents, the reason for this
//! duplication is because sorbit needs to work in `no_std` environments, where
//! these traits aren't available.
//!
//! In the [io] modules, sorbit already provides some in-memory streams for
//! both `alloc` and no `alloc` environments. You can implement your own streams
//! as necessary.
//!
//! ## Multi-pass serialization
//!
//! Regular `Serializer`s write the output bytes monotonously, without ever
//! looking back at the bytes written. This works for most cases, but some items,
//! like checksums, are calculated after the object has been serialized. To solve
//! this issue, sorbit also provides the [ser_de::MultiPassSerialize] and
//! [ser_de::RevisableSerializer] traits. With these traits, it's possible to
//! look back at the previously serialized data, make calculations, and update
//! parts of or all of the previously written bytes.
//!
//! ## Deriving serialization with macros and attributes
//!
//! Sorbit can derive the serialization and deserialization implementations
//! for `struct`s and `enum`s, and makes available directives to control
//! the layout.
//!
//! The directives can be merged (e.g. `#[sorbit(offset=4, round=4)]`) or
//! written separately (e.g. `#[sorbit(offset=4)] #[sorbit(round=4)]`).
//!
//! ### Structures
//!
//! Structures, fields, and bit fields support the following directives:
//!
//! ```
//! use sorbit::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! #[sorbit(byte_order=big_endian)]
//! #[sorbit(len=22)]
//! #[sorbit(round=8)]
//! struct Example {
//!     #[sorbit(byte_order=little_endian)]
//!     #[sorbit(offset=6)]
//!     #[sorbit(align=4)]
//!     #[sorbit(round=4)]
//!     field: u8,
//!     
//!     #[sorbit(bit_field=_bit_field)]
//!     #[sorbit(byte_order=little_endian)]
//!     #[sorbit(offset=12)]
//!     #[sorbit(align=4)]
//!     #[sorbit(round=4)]
//!     #[sorbit(bit_numbering=LSB0)]
//!     #[sorbit(repr=u8)]
//!     #[sorbit(bits=1..=3)]
//!     member_one: u8,
//!     #[sorbit(bit_field=_bit_field)]
//!     #[sorbit(bits=4..=6)]
//!     member_two: u8,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct CollectionByLen {
//!     #[sorbit(value=len(items))]
//!     len: u8,
//!     #[sorbit(value=len_by(len))]
//!     items: Vec<u8>,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct CollectionByByteCount {
//!     #[sorbit(value=byte_count(items))]
//!     byte_count: u8,
//!     #[sorbit(value=byte_count_by(byte_count))]
//!     items: Vec<u8>,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct MultiPassField {
//!     #[sorbit(multi_pass)]
//!     field: CollectionByByteCount,
//! }
//! ```
//!
//! #### The structure itself
//!
//! | Directive     | Values                        | Description |
//! |---------------|-------------------------------|-------------|
//! | `byte_order`  | `big_endian`, `little_endian` | The default byte ordering for all fields and bit fields. If not present, the byte order is inherited from the enclosing structure. |
//! | `len`         | Any positive integer          | The structure's total length in bytes. If the serialized structure is smaller, it is padded to this length, if larger, this is ignored. |
//! | `round`       | Any positive integer          | The structure's total length is padded to be a multiple of this value. Will pad beyond the requested `len` to satisfy rounding. |
//!
//! #### Fields
//!
//! | Directive     | Values                        | Description |
//! |---------------|-------------------------------|-------------|
//! | `byte_order`  | `big_endian`, `little_endian` | The byte ordering of this specific field. When present, overrides the ordering inherited from the structure. |
//! | `offset`      | Any positive integer          | The offset from the beginning of the structure where this field begins. An error is raised during serialization if the offset is already occupied. |
//! | `align`       | Any positive integer          | The offset from the beginning of the structure will be a multiple of `align`. Zero padding is applied before the field, as necessary. |
//! | `round`       | Any positive integer          | The field's length is zero-padded to be a multiple of this value. |
//! | `value`       | Expression (see below)        | Ignore the field's value, and use the value provided by the expression. |
//! | `multi_pass`  | None, true, false             | A marker attribute to tell sorbit that the field only implements [`MultiPassSerialize`](crate::ser_de::MultiPassSerialize), but not [`Serialize`](crate::ser_de::Serialize). Apply it only when necessary. This marker *is* indeed superfluous, but proc macros cannot look into the type system, and generic programming is not quite there yet. |
//!
//! Value expressions:
//!
//! | Expression               | Description |
//! |--------------------------|-------------|
//! | `value=len(c)`           | The serialized value will be the length of `self.c`, calculated as `(&self.c).into_iterator().len()`. This implies that `&self.c` supports [`IntoIterator`] and the iterator is an [`ExactSizeIterator`]. During deserialization, `self.c` will contain the number of items as per this field. Deserialization requires `self.c` to implement [`FromIterator`]. |
//! | `value=len_by(l)`        | The length of this collection is serialized as `self.l`. This is the sibling attribute of `value=len(c)`, and it's enough if you specify only one of them. |
//! | `value=byte_count(c)`    | The serialized value will be the number of bytes the serialized items of `self.c` occupy altogether. For deserialization, `self.c` has to implement [`FromIterator`]. Using this attribute will make the structure only [`MultiPassSerialize`](crate::ser_de::MultiPassSerialize). |
//! | `value=byte_count_by(b)` | The number of bytes the serialized items of this field occupy together is serialized as `self.b`. This is the sibling attribute of `value=byte_count(c)`, and it's enough if you specify only one of them. |
//! | `value=constant(expr)`   | The item's value will always be `expr` when serializing, and deserialization will fail if the value is not `expr`. The `expr` must be convertible to the field's type via [`From`]. |
//!
//! #### Bit fields
//!
//! Bit fields in sorbit are defined using two concepts:
//! - Bit field *storage*: The storage is a virtual field of the struct. It is
//!   virtual, because it's not an actual field in the Rust code, it only exists
//!   for serialization. Bit field storages are declared via attributes:
//!   `#[sorbit(bit_field=<NAME>, repr=<TYPE>)]`. This declares a storage of the
//!   name `<NAME>`, with the type `<TYPE>`.
//! - Bit field *member*: An actual field of the struct that is narrowed down
//!   to a arbitrary width and is placed into a bit field storage during
//!   serialization. Any field with the `bit_field` meta attribute is interpreted
//!   as a bit field. Additionally, the `#[sorbit(bits=<BITS>)]` attribute must
//!   also be present.
//!
//! The storages are not explicitly defined: whenever sorbit encounters a new
//! `bit_field` meta attribute, it creates a storage for it. All bit field
//! members that use the same `bit_field` meta attribute are part of the same
//! bit field storage. Members of the same storage must appear consecutively
//! in the `struct`, and are replaced by the storage at serialization.
//!
//! As an example, imagine this structure with one bit field storage that has
//! two members:
//! ```
//! # use sorbit::{Serialize, Deserialize};
//! #
//! #[derive(Serialize, Deserialize)]
//! struct Rust {
//!     #[sorbit(bit_field=_bit_field, repr=u16)]
//!     #[sorbit(byte_order=little_endian)]
//!     #[sorbit(bits=1..=5)]
//!     one: u8,
//!     #[sorbit(bit_field=_bit_field)]
//!     #[sorbit(bits=9..=12)]
//!     two: u8,
//! }
//! ```
//!
//! From the serialization perspective, it's interpreted like this:
//! ```
//! # use sorbit::{Serialize, Deserialize};
//! #
//! #[derive(Serialize, Deserialize)]
//! struct Binary {
//!     #[sorbit(byte_order=little_endian)]
//!     _bit_field: u16,
//! }
//! ```
//!
//! | Directive       | Values                        | Description |
//! |-----------------|-------------------------------|-------------|
//! | `byte_order`    | `big_endian`, `little_endian` | The byte ordering of the bit field storage. Same as for regular fields. |
//! | `offset`        | Any positive integer          | The offset of the bit field storage. Same as for regular fields. |
//! | `align`         | Any positive integer          | The alignment of the bit field storage. Same as for regular fields. |
//! | `round`         | Any positive integer          | The rounding of the bit field storage. Same as for regular fields. |
//! | `bit_numbering` | `LSB0` (default), `MSB0`      | The bit numbering of all members of the storage. With `LSB0`, bit `0` refers to the least significant bit, and `MSB0` is the opposite. Note that this does not affect the serialized format, it merely affects the number you write for the `bits` meta attribute of bit field members. |
//! | `repr`          | Any type                      | The type of the bit field storage. |
//! | `bits`          | Bounded range (`bits=a..b`, `bits=a..=b`), number (`bits=a`) | The bits occupied by the member within the storage. The values must be integer literals. |
//!
//! While both the bit field members and the bit field storage may be any types,
//! they are linked by the [`bit::PackInto`] and [`bit::UnpackFrom`] traits.
//! As long as these traits are implement for the member-storage type pair,
//! the serialization can be derived.
//!
//! #### Phantom fields
//!
//! You can use [`PhantomData<T>`](std::marker::PhantomData) for any field or
//! computed bit field member of the struct. When no value-altering attributes
//! (e.g. `value=constant(0)`) are present, the phantom field will not be
//! serialized. When the value is computed by an attribute, the computed value
//! will be serialized as usual.
//!
//! The main use case of phantom fields is to ignore computed fields at runtime:
//!
//! ```
//! # use sorbit::{Serialize, Deserialize};
//! # use std::marker::PhantomData;
//! #
//! #[derive(Serialize, Deserialize)]
//! struct Collection {
//!     #[sorbit(value=len(data))]
//!     length: PhantomData<u16>,
//!     data: Vec<u8>,
//! }
//! ```
//!
//! ### Enumerations
//!
//! #### Serialization
//!
//! ```
//! use sorbit::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! #[sorbit(byte_order=big_endian)]
//! #[repr(u8)]
//! enum Example {
//!     A = 1,
//!     B(u32),
//!     C{c: u16},
//!     #[sorbit(catch_all)]
//!     D(u8),
//! }
//! ```
//!
//! | Directive       | Values                         | Description |
//! |-----------------|--------------------------------|-------------|
//! | `byte_order`    | `big_endian`, `little_endian`  | The byte ordering of the enum's discriminant, as well as the values in the enum's fields. The latter can be overridden by attributes on the variant itself. |
//! | `repr`          | A primitive type               | The type used to represent and serialize the discriminant. See the [language documentation](https://doc.rust-lang.org/nomicon/other-reprs.html). |
//! | `catch_all`     | - (`true` or `false` accepted) | Mark the variant as a catch all for unrecognized discriminant during deserialization. |
//!
//! The enum's repr is chosen as `isize` unless specified otherwise. This
//! follows the Rust language's specification.
//!
//! The enum's variants may have unnamed (tuple-like) and named (struct-like)
//! fields. In such cases, the variant is treated and parsed as a `struct`. All
//! rules of `struct` serialization apply, so the variant can use all attributes
//! related to structs and can also have bit fields.
//!
//! In the case of variants with fields, the discriminant is first serialized
//! as usual, and it is followed immediately by the variant, which is serialized
//! as a `struct``.
//!
//! The catch all variant may be a unit variant (i.e. `CatchAll`) or a tuple
//! variant with exactly one field of the same type as the enum's repr
//! (i.e. `CatchAll(u16)`). When the catch all is a tuple field, the
//! discriminant read during deserialization will be stored in the catch all
//! variant. During serialization, when the enum is the catch all variant,
//! the discriminant is taken from the catch all variant's field.
//!
//! #### Bit packing
//!
//! Remember the [sorbit::bit::PackInto] and [sorbit::bit::UnpackFrom] traits
//! you need to implement for a type to use it in a bit field?
//!
//! You can derive them for enumerations:
//!
//! ```
//! use sorbit::{PackInto, UnpackFrom};
//!
//! #[derive(PackInto, UnpackFrom)]
//! #[sorbit(byte_order=big_endian)]
//! #[repr(u8)]
//! enum Example {
//!     A = 1,
//!     #[sorbit(catch_all)]
//!     B(u8),
//! }
//! ```
//!
//! Once derived, you can use the enumeration in bit fields. Keep in mind that
//! the packing is forwarded to the enum's repr type. The `catch_all` attribute
//! is handled the same way as for serialization, other attributes are ignored.
//!
//! The derivation of bit packing is only applicable to unit enums. You can
//! still derive the traits by hand if it makes sense for you.
//!
//! ## `no_std`
//!
//! Sorbit is designed to fully support `no_std` and no `alloc` environments.
//! None of the traits or other design elements require memory allocations.
//!
//! Sorbit ships with a memory stream implementation that uses allocation, but
//! non-allocating memory streams are also included. Both streams can be used
//! together with the stream serializers.
//!
//! ## Sorbit vs. ...
//!
//! *To the authors of the libraries mentioned below, please reach out if you
//! find inaccurate information.*
//!
//! - Sorbit vs [serde](https://docs.rs/serde/latest/serde/): Serde is a general
//!   serialization framework, excelling at handling all sorts of text formats
//!   (e.g. JSON, YAML) and efficient binary formats (e.g. [postcard](https://docs.rs/postcard/latest/postcard/)).
//!   Sorbit, on the other hand, is tailored to define specific binary formats,
//!   which is less convenient, if at all possible, with serde. While sorbit is
//!   also very efficient, crates like serde provide way more options together
//!   with a first-class support for textual formats.
//! - Sorbit vs [deku](https://docs.rs/deku/0.20.3/deku/index.html): Regarding
//!   functionality, sorbit and deku are very similar, as they both focus on
//!   making the serialization of specific binary formats easier.
//!
//!   So what are the differences then?
//!   - While deku supports `no_std` environments, sorbit also supports heapless
//!     environments.
//!   - The APIs look and feel different. You should take a look at both before
//!     making your mind up.

// Disable the [`std`] standard crate when the "std" feature is not enabled.
#![cfg_attr(not(feature = "std"), no_std)]

// Enable the [`alloc`] standard crate when the "alloc" feature is enabled.
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bit;
pub mod byte_order;
pub mod error;
pub mod io;
pub mod ser_de;
pub use sorbit_derive::{Deserialize, PackInto, Serialize, UnpackFrom};
pub mod collection;
pub mod stream_ser_de;

mod types;

extern crate self as sorbit;
