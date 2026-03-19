#[cfg(feature = "alloc")]
use alloc::vec::Vec;

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
    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> Result<Vec<u8>, Error>;
    /// Serialize the value into a blob of bytes.
    fn to_byte_slice(&self, bytes: &mut [u8]) -> Result<(), Error>;
}

impl<T> ToBytes<false> for T
where
    T: Serialize,
{
    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        use crate::io::GrowingMemoryStream;
        use crate::stream_ser_de::StreamSerializer;

        let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
        self.serialize(&mut serializer)?;
        Ok(serializer.take().take())
    }

    fn to_byte_slice(&self, bytes: &mut [u8]) -> Result<(), Error> {
        let mut serializer = StreamSerializer::new(FixedMemoryStream::new(bytes));
        self.serialize(&mut serializer).map(|_| ())
    }
}

impl<T> ToBytes<true> for T
where
    T: MultiPassSerialize,
{
    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        use crate::io::GrowingMemoryStream;
        use crate::stream_ser_de::StreamSerializer;

        let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
        self.serialize(&mut serializer)?;
        Ok(serializer.take().take())
    }

    fn to_byte_slice(&self, bytes: &mut [u8]) -> Result<(), Error> {
        let mut serializer = StreamSerializer::new(FixedMemoryStream::new(bytes));
        self.serialize(&mut serializer).map(|_| ())
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
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error>;
}

impl<T> FromBytes for T
where
    T: Deserialize,
{
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut deserializer = StreamDeserializer::new(FixedMemoryStream::new(bytes));
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
    fn to_byte_single_pass() {
        let value = SinglePass;
        assert_eq!(value.to_bytes(), Ok(vec![]));
    }

    #[test]
    fn to_byte_multi_pass() {
        let value = MultiPass;
        assert_eq!(value.to_bytes(), Ok(vec![]));
    }
}
