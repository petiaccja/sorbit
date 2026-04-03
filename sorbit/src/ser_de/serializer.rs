use core::convert::Infallible;

use crate::bit::Error as BitError;
use crate::byte_order::ByteOrder;
use crate::error::{MessageError, TraceError};
use crate::io::Read;

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

/// Serializers can transform primitive types into a stream of bytes that can
/// be sent over the network or stored in files.
pub trait Serializer {
    /// The type a [`Serializer`] returns if serialization succeeded.
    type Success;
    /// The type a [`Serializer`] returns if serialization failed.
    type Error: TraceError + MessageError + From<BitError>;

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

    /// Serialize an [`u128`] value according to the current byte order.
    fn serialize_u128(&mut self, value: u128) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i8`] value.
    fn serialize_i8(&mut self, value: i8) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i16`] value according to the current byte order.
    fn serialize_i16(&mut self, value: i16) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i32`] value according to the current byte order.
    fn serialize_i32(&mut self, value: i32) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i64`] value according to the current byte order.
    fn serialize_i64(&mut self, value: i64) -> Result<Self::Success, Self::Error>;

    /// Serialize an [`i128`] value according to the current byte order.
    fn serialize_i128(&mut self, value: i128) -> Result<Self::Success, Self::Error>;

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
    /// of the current composite. (See [`serialize_composite`](Self::serialize_composite).)
    ///
    /// ## Errors
    ///
    /// When the stream has already been written past `until`, an error is
    /// returned.
    fn pad(&mut self, until: u64) -> Result<Self::Success, Self::Error>;

    /// Pad with zeros so that the size of the current composite becomes a
    /// multiple of `multiple_of`. (See [`serialize_composite`](Self::serialize_composite).)
    fn align(&mut self, multiple_of: u64) -> Result<Self::Success, Self::Error>;

    /// Serialize a composite object (e.g. a struct).
    ///
    /// This does not affect the underlying stream and serves only as a marker
    /// for the [`pad`](Self::pad) and [`align`](Self::align) functions.
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
        serialize_members: impl FnOnce(&mut Self) -> Result<Output, Self::Error>,
    ) -> Result<(Self::Success, Output), Self::Error>;

    /// Temporarily change the byte order.
    ///
    /// All items serialized in the `serialize_members` function will use the
    /// selected byte order. This call can be nested as necessary.
    fn with_byte_order<Output>(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self) -> Result<Output, Self::Error>,
    ) -> Result<Output, Self::Error>;

    /// Return [`Ok`].
    ///
    /// Use this to exit serialization with a success when you don't have any
    /// actual values to serialize.
    fn success(&mut self) -> Result<Self::Success, Self::Error>;

    /// Return an error, indicating that serialization failed.
    ///
    /// This method can be called by implementors of [`Serialize`](crate::ser_de::Serialize)
    /// when an error occurs during serialization.
    fn error(&mut self, message: &'static str) -> Result<Infallible, Self::Error>;
}

/// A serializer that can analyze and update previously serialized objects.
///
/// When serializing an object succeeds, revisable serializers always return
/// a [Span] that contains the location in the stream where the object was
/// serialized. The span can later be used to analyze and update the stream.
pub trait RevisableSerializer: Serializer<Success: Span> {
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
    fn analyze_span<Output, Error, AnalyzeSpanFn>(
        &mut self,
        span: &Self::Success,
        analyze_span_fn: AnalyzeSpanFn,
    ) -> Result<Output, Self::Error>
    where
        AnalyzeSpanFn: for<'analyze> FnOnce(&mut dyn Read) -> Result<Output, Error>,
        Error: Into<Self::Error>;

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
    /// lengths can be computed by [`analyze_span`](Self::analyze_span).
    fn revise_span<Output>(
        &mut self,
        span: &Self::Success,
        serialize_span: impl FnOnce(&mut Self) -> Result<Output, Self::Error>,
    ) -> Result<Output, Self::Error>;
}
