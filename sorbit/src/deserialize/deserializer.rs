use crate::{byte_order::ByteOrder, error::SerializeError};

/// Derializers can transform a stream of bytes that can
/// be sent over the network or stored in files into primitive types.
pub trait Deserializer: Sized {
    /// The error type returned upon deserialization failure.
    type Error: SerializeError;

    /// The type of the deserializer passed to the member deserializer in
    /// [`Self::deserialize_composite`].
    type CompositeDeserializer: Deserializer<Error = Self::Error>;

    /// The type of the deserializer passed to the member deserializer in
    /// [`Self::with_byte_order`].
    type ByteOrderDeserializer: Deserializer<Error = Self::Error>;

    /// Deserialize a [`bool`] value.
    fn deserialize_bool(&mut self) -> Result<bool, Self::Error>;

    /// Deserialize a [`u8`] value.
    fn deserialize_u8(&mut self) -> Result<u8, Self::Error>;

    /// Deserialize a [`u16`] value according the current byte order.
    fn deserialize_u16(&mut self) -> Result<u16, Self::Error>;

    /// Deserialize a [`u32`] value according the current byte order.
    fn deserialize_u32(&mut self) -> Result<u32, Self::Error>;

    /// Deserialize a [`u64`] value according the current byte order.
    fn deserialize_u64(&mut self) -> Result<u64, Self::Error>;

    /// Deserialize a [`i8`] value.
    fn deserialize_i8(&mut self) -> Result<i8, Self::Error>;

    /// Deserialize a [`i16`] value according the current byte order.
    fn deserialize_i16(&mut self) -> Result<i16, Self::Error>;

    /// Deserialize a [`i32`] value according the current byte order.
    fn deserialize_i32(&mut self) -> Result<i32, Self::Error>;

    /// Deserialize a [`i64`] value according the current byte order.
    fn deserialize_i64(&mut self) -> Result<i64, Self::Error>;

    /// Deserialize a [`u8`] array.
    ///
    /// The size of the array should **not** be stored in the byte stream
    /// for deserializers that aim to support bit-exact representations.
    /// The caller is expected to deserialize knowing the array's size
    /// at compilation time.
    fn deserialize_array<const N: usize>(&mut self) -> Result<[u8; N], Self::Error>;

    /// Deserialize an [`u8`] slice.
    ///
    /// The size of the slice should **not** be stored in the byte stream for
    /// deserializers that aim to support bit-exact representations. The caller
    /// is expected to deserialize the size separately as it's represented in the
    /// serialized data structure's specification.
    fn deserialize_slice(&mut self, value: &mut [u8]) -> Result<(), Self::Error>;

    /// Pad with zeros up to `until`, which is interpreted from the beginning
    /// of the current composite. (See [`Self::deserialize_composite`].)
    ///
    /// ## Errors
    ///
    /// When the stream has already been written past `until`, an error is
    /// returned.
    fn pad(&mut self, until: u64) -> Result<(), Self::Error>;

    /// Pad with zeros so that the size of the current composite becomes a
    /// multiple of `multiple_of`. (See [`Self::deserialize_composite`].)
    fn align(&mut self, multiple_of: u64) -> Result<(), Self::Error>;

    /// Deserialize a composite object (e.g. a struct).
    ///
    /// This does not affect the underlying stream and serves only as a marker
    /// for the [`Self::pad`] and [`Self::align`] functions.
    /// This call can be nested as necessary (i.e. composite of composites).
    ///
    /// ## Members of the composite
    ///
    /// The `deserialize_members` function should take care of deserializing the
    /// members of the composite object (e.g. fields of a struct).
    ///
    /// ## Returned value
    ///
    /// The result from `deserialize_members` is returned as is.
    fn deserialize_composite<O>(
        &mut self,
        deserialize_members: impl FnOnce(&mut Self::CompositeDeserializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error>;

    /// Temporarily change the byte order.
    ///
    /// All items serialized in the `deserialize_members` function will use the
    /// selected byte order. This call can be nested as necessary.
    fn with_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        deserialize_members: impl FnOnce(&mut Self::ByteOrderDeserializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error>;
}
