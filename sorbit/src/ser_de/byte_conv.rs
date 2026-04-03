#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
use crate::byte_order::ByteOrder;
use crate::error::Error;
use crate::io::FixedMemoryStream;
use crate::ser_de::{Deserialize, MultiPassSerialize, Serialize};
use crate::stream_ser_de::{StreamDeserializer, StreamSerializer};

/// Serialize a value to a blob of bytes.
///
/// This is a utility trait that saves you the hassle of instantiating a
/// serializer, serializing the object, and retrieving the bytes.
///
/// This trait is blanket implemented for every type that implements [Serialize]
/// or [MultiPassSerialize].
pub trait ToBytes<const MULTI_PASS: bool> {
    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is native by default, but it may be overridden by
    /// the data structure.
    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.to_xe_bytes(ByteOrder::native())
    }

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is big endian by default, but it may be overridden by
    /// the data structure.
    #[cfg(feature = "alloc")]
    fn to_be_bytes(&self) -> Result<Vec<u8>, Error> {
        self.to_xe_bytes(ByteOrder::BigEndian)
    }

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is little endian by default, but it may be overridden by
    /// the data structure.
    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> Result<Vec<u8>, Error> {
        self.to_xe_bytes(ByteOrder::LittleEndian)
    }

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is as specified by default, but it may be overridden by
    /// the data structure.
    #[cfg(feature = "alloc")]
    fn to_xe_bytes(&self, byte_order: ByteOrder) -> Result<Vec<u8>, Error>;

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is native by default, but it may be overridden by
    /// the data structure.
    fn to_byte_slice<'b>(&self, bytes: &'b mut [u8]) -> Result<&'b mut [u8], Error> {
        self.to_xe_byte_slice(bytes, ByteOrder::native())
    }

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is big endian by default, but it may be overridden by
    /// the data structure.
    fn to_be_byte_slice<'b>(&self, bytes: &'b mut [u8]) -> Result<&'b mut [u8], Error> {
        self.to_xe_byte_slice(bytes, ByteOrder::BigEndian)
    }

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is little endian by default, but it may be overridden by
    /// the data structure.
    fn to_le_byte_slice<'b>(&self, bytes: &'b mut [u8]) -> Result<&'b mut [u8], Error> {
        self.to_xe_byte_slice(bytes, ByteOrder::LittleEndian)
    }

    /// Serialize the value into a blob of bytes.
    ///
    /// The byte order is as specified by default, but it may be overridden by
    /// the data structure.
    fn to_xe_byte_slice<'b>(&self, bytes: &'b mut [u8], byte_order: ByteOrder) -> Result<&'b mut [u8], Error>;
}

impl<T> ToBytes<false> for T
where
    T: Serialize,
{
    #[cfg(feature = "alloc")]
    fn to_xe_bytes(&self, byte_order: ByteOrder) -> Result<Vec<u8>, Error> {
        use crate::io::GrowingMemoryStream;
        use crate::stream_ser_de::StreamSerializer;

        let mut serializer = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(byte_order);
        self.serialize(&mut serializer)?;
        Ok(serializer.take().take())
    }

    fn to_xe_byte_slice<'b>(&self, bytes: &'b mut [u8], byte_order: ByteOrder) -> Result<&'b mut [u8], Error> {
        let mut serializer = StreamSerializer::new(FixedMemoryStream::new(bytes)).change_byte_order(byte_order);
        self.serialize(&mut serializer).map(move |_| serializer.take().take())
    }
}

impl<T> ToBytes<true> for T
where
    T: MultiPassSerialize,
{
    #[cfg(feature = "alloc")]
    fn to_xe_bytes(&self, byte_order: ByteOrder) -> Result<Vec<u8>, Error> {
        use crate::io::GrowingMemoryStream;
        use crate::stream_ser_de::StreamSerializer;

        let mut serializer = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(byte_order);
        self.serialize(&mut serializer)?;
        Ok(serializer.take().take())
    }

    fn to_xe_byte_slice<'b>(&self, bytes: &'b mut [u8], byte_order: ByteOrder) -> Result<&'b mut [u8], Error> {
        let mut serializer = StreamSerializer::new(FixedMemoryStream::new(bytes)).change_byte_order(byte_order);
        self.serialize(&mut serializer).map(move |_| serializer.take().take())
    }
}

/// Deserialize a value from a blob of bytes.
///
/// This is a utility trait that saves you the hassle of instantiating a
/// deserializer and deserializing the object.
///
/// This trait is blanket implemented for every type that implements
/// [Deserialize].
pub trait FromBytes: Sized {
    /// Deserialize a value from a blob of bytes.
    ///
    /// The byte order is native by default, but it may be overridden by
    /// the data structure.
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_xe_bytes(bytes, ByteOrder::native())
    }

    /// Deserialize a value from a blob of bytes.
    ///
    /// The byte order is big endian by default, but it may be overridden by
    /// the data structure.
    fn from_be_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_xe_bytes(bytes, ByteOrder::BigEndian)
    }

    /// Deserialize a value from a blob of bytes.
    ///
    /// The byte order is little endian by default, but it may be overridden by
    /// the data structure.
    fn from_le_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_xe_bytes(bytes, ByteOrder::LittleEndian)
    }

    /// Deserialize a value from a blob of bytes.
    ///
    /// The byte order is as specified by default, but it may be overridden by
    /// the data structure.
    fn from_xe_bytes(bytes: &[u8], byte_order: ByteOrder) -> Result<Self, Error>;
}

impl<T> FromBytes for T
where
    T: Deserialize,
{
    fn from_xe_bytes(bytes: &[u8], byte_order: ByteOrder) -> Result<Self, Error> {
        let mut deserializer = StreamDeserializer::new(FixedMemoryStream::new(bytes)).change_byte_order(byte_order);
        Self::deserialize(&mut deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ser_de::Serialize;

    struct SinglePass;

    impl Serialize for SinglePass {
        fn serialize<S: crate::ser_de::Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
            serializer.success()
        }
    }

    struct MultiPass;

    impl Serialize for MultiPass {
        fn serialize<S: crate::ser_de::Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
            serializer.success()
        }
    }

    #[test]
    fn to_bytes_single_pass() {
        let value = SinglePass;
        assert_eq!(value.to_bytes(), Ok(vec![]));
    }

    #[test]
    fn to_bytes_multi_pass() {
        let value = MultiPass;
        assert_eq!(value.to_bytes(), Ok(vec![]));
    }

    #[test]
    fn to_byte_endianness() {
        let value = 0xABCD_u16;
        let be_bytes = value.to_be_bytes();
        let le_bytes = value.to_le_bytes();
        let native_bytes = value.to_ne_bytes();
        assert_eq!(ToBytes::to_bytes(&value).unwrap(), native_bytes);
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), be_bytes);
        assert_eq!(ToBytes::to_le_bytes(&value).unwrap(), le_bytes);
        assert_eq!(ToBytes::to_xe_bytes(&value, ByteOrder::BigEndian).unwrap(), be_bytes);
        assert_eq!(ToBytes::to_xe_bytes(&value, ByteOrder::LittleEndian).unwrap(), le_bytes);
        let mut buffer = [0u8; 2];
        assert_eq!(ToBytes::to_byte_slice(&value, &mut buffer).unwrap(), native_bytes);
        assert_eq!(ToBytes::to_be_byte_slice(&value, &mut buffer).unwrap(), be_bytes);
        assert_eq!(ToBytes::to_le_byte_slice(&value, &mut buffer).unwrap(), le_bytes);
        assert_eq!(ToBytes::to_xe_byte_slice(&value, &mut buffer, ByteOrder::BigEndian).unwrap(), be_bytes);
        assert_eq!(ToBytes::to_xe_byte_slice(&value, &mut buffer, ByteOrder::LittleEndian).unwrap(), le_bytes);
    }
}
