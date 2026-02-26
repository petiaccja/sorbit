use crate::bit::Error as BitError;
use crate::byte_order::ByteOrder;
use crate::error::TraceError;
use crate::io::{Read, Seek};

/// The section of the byte stream where a serialized object resides.
///
/// For example, for the [IPv4 header](https://en.wikipedia.org/wiki/IPv4#Header),
/// the section that belongs to *Time to Live* would be bytes 8 to 9 (non-inclusive).
pub trait Span {
    /// Return the length of the span in bytes.
    fn len(&self) -> u64;
    /// Return the byte offset into the stream where the span starts. Inclusive.
    fn start(&self) -> u64;
    /// Return the byte offset into the stream where the span ends. Exclusive.
    fn end(&self) -> u64;
}

/// A helper trait to define the types a [`Serializer`] returns on success
/// and error.
pub trait SerializerOutput {
    /// The type a [`Serializer`] returns if serialization succeeded.
    type Success;
    /// The type a [`Serializer`] returns if serialization failed.
    type Error: TraceError + From<BitError>;
}

/// Serializers can transform primitive types into a stream of bytes that can
/// be sent over the network or stored in files.
pub trait Serializer: SerializerOutput {
    /// The type of the serializer passed to the member serializer in
    /// [`Serializer::serialize_composite`].
    type CompositeSerializer: Serializer<Success = Self::Success, Error = Self::Error>;

    /// The type of the serializer passed to the member serializer in
    /// [`Serializer::with_byte_order`].
    type ByteOrderSerializer: Serializer<Success = Self::Success, Error = Self::Error>;

    /// Serialize a fictional 0-byte object. Useful for producing a result
    /// (typically [`SerializerOutput::Success`]) when doing generic programming.
    fn serialize_nothing(&mut self) -> Result<Self::Success, Self::Error>;

    /// Serialize a [`bool`] value.
    fn serialize_bool(&mut self, value: bool) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`u8`] value.
    fn serialize_u8(&mut self, value: u8) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`u16`] value according to the current byte order.
    fn serialize_u16(&mut self, value: u16) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`u32`] value according to the current byte order.
    fn serialize_u32(&mut self, value: u32) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`u64`] value according to the current byte order.
    fn serialize_u64(&mut self, value: u64) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i8`] value.
    fn serialize_i8(&mut self, value: i8) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i16`] value according to the current byte order.
    fn serialize_i16(&mut self, value: i16) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i32`] value according to the current byte order.
    fn serialize_i32(&mut self, value: i32) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i64`] value according to the current byte order.
    fn serialize_i64(&mut self, value: i64) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`u8`] array.
    ///
    /// The size of the array should **not** be stored in the byte stream
    /// for serializers that aim to support bit-exact representations.
    /// The caller is expected to deserialize knowing the array's size
    /// at compilation time.
    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`u8`] slice.
    ///
    /// The size of the slice should **not** be stored in the byte stream for
    /// serializers that aim to support bit-exact representations. The caller
    /// is expected to serialize the size separately as it's represented in the
    /// serialized data structure's specification.
    fn serialize_slice(&mut self, value: &[u8]) -> Result<Self::Success, Self::Error>;

    /// Pad with zeros up to `until`, which is interpreted from the beginning
    /// of the current composite. (See [`Serializer::serialize_composite`].)
    ///
    /// ## Errors
    ///
    /// When the stream has already been written past `until`, an error is
    /// returned.
    fn pad(&mut self, until: u64) -> Result<Self::Success, Self::Error>;

    /// Pad with zeros so that the size of the current composite becomes a
    /// multiple of `multiple_of`. (See [`Serializer::serialize_composite`].)
    fn align(&mut self, multiple_of: u64) -> Result<Self::Success, Self::Error>;

    /// Serialize a composite object (e.g. a struct).
    ///
    /// This does not affect the underlying stream and serves only as a marker
    /// for the [`Serializer::pad`] and [`Serializer::align`] functions.
    /// This call can be nested as necessary (i.e. composite of composites).
    ///
    /// ## Members of the composite
    ///
    /// The `serialize_members` function should take care of serializing the
    /// members of the composite object (e.g. fields of a struct).
    ///
    /// ## Returned value
    ///
    /// A tuple of the [`Span`] of the entire composite and the output of `serialize_members`.
    fn serialize_composite<Output>(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::CompositeSerializer) -> Result<Output, Self::Error>,
    ) -> Result<(Self::Success, Output), Self::Error>;

    /// Temporarily change the byte order.
    ///
    /// All items serialized in the `serialize_members` function will use the
    /// selected byte order. This call can be nested as necessary.
    fn with_byte_order<Output>(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self::ByteOrderSerializer) -> Result<Output, Self::Error>,
    ) -> Result<(Self::Success, Output), Self::Error>;
}

/// A serializer that can introspect previously serialized objects.
pub trait Lookback: SerializerOutput<Success: Span> {
    /// The type of the serializer passed to the update function in
    /// [`Lookback::update_section`].
    type SectionSerializer: Serializer + Lookback<Success = Self::Success, Error = Self::Error>;
    /// The type of the stream passed to the analyzer function in
    /// [`Lookback::analyze_section`].
    type SectionReader: Read + Seek;

    /// Analyze the byte stream of previously serialized items.
    ///
    /// Parameters:
    /// - `section`: which bytes of the output stream to analyze. Sections are
    ///   returned by previous calls to the `serialize_*` functions.
    /// - `analyze_bytes`: a function that analyzes the stream containing only
    ///   the bytes defined by the section.
    ///
    /// This function can be used to compute checksums or to measure the final
    /// length of a data structure. These calculations cannot easily be done
    /// on the unserialized object as they require the raw bytes.
    fn analyze_section<Output>(
        &mut self,
        section: &Self::Success,
        analyze_bytes: impl FnOnce(&mut Self::SectionReader) -> Output,
    ) -> Result<Output, Self::Error>;

    /// Update the bytes belonging to a previously serialized item.
    ///
    /// Parameters:
    /// - `section`: which bytes of the output stream to update. Sections are
    ///   returned by previous calls to the `serialize_*` functions.
    /// - `update_section`: a function that takes a serializer that overwrites
    ///   the bytes defined by the `section`. Use this section serialized to
    ///   update the bytes of previously serialized items.
    ///
    /// This function can be used to write checksums of the length of items
    /// after the rest of the structure was serialized. The checksums or
    /// lengths can be computed by [`Lookback::analyze_section`].
    fn update_section<Output>(
        &mut self,
        section: &Self::Success,
        update_section: impl FnOnce(&mut Self::SectionSerializer) -> Result<Output, Self::Error>,
    ) -> Result<Output, Self::Error>;
}

/// A multi-pass serializer is a special [`Serializer`] that can look back at the
/// previously serialized bytes and change them.
///
/// Some types cannot be serialized in a single pass. Think about the IHL and
/// the checksum fields in the IPv4 header. In the first pass, you need to
/// serialize the header with IHL and checksum set to zero. In the second pass,
/// you need to look back at the entire serialized header to determine its length
/// in bytes, and reserialize the IHL accordingly. After this, a third pass is
/// needed, looking back at the entire byte span of the serialized header to
/// calculate the checksum, and then the checksum needs to be reserialized.
///
/// In addition to the regular [`Serializer`] methods, `MultiPassSerializer`s
/// also implement [`Lookback`] so that you can review and update the serialized
/// byte stream.
pub trait MultiPassSerializer:
    Serializer<CompositeSerializer: Lookback, ByteOrderSerializer: Lookback> + Lookback
{
}

impl<S> MultiPassSerializer for S where
    S: Serializer<CompositeSerializer: Lookback, ByteOrderSerializer: Lookback> + Lookback
{
}
