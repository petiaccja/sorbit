use crate::{byte_order::ByteOrder, error::SerializeError};

pub trait Deserializer: Sized {
    type Error: SerializeError;
    type CompositeDeserializer: Deserializer<Error = Self::Error>;
    type ByteOrderDeserializer: Deserializer<Error = Self::Error>;

    fn deserialize_bool(&mut self) -> Result<bool, Self::Error>;
    fn deserialize_u8(&mut self) -> Result<u8, Self::Error>;
    fn deserialize_u16(&mut self) -> Result<u16, Self::Error>;
    fn deserialize_u32(&mut self) -> Result<u32, Self::Error>;
    fn deserialize_u64(&mut self) -> Result<u64, Self::Error>;
    fn deserialize_i8(&mut self) -> Result<i8, Self::Error>;
    fn deserialize_i16(&mut self) -> Result<i16, Self::Error>;
    fn deserialize_i32(&mut self) -> Result<i32, Self::Error>;
    fn deserialize_i64(&mut self) -> Result<i64, Self::Error>;
    fn deserialize_array<const N: usize>(&mut self) -> Result<[u8; N], Self::Error>;
    fn deserialize_slice(&mut self, value: &mut [u8]) -> Result<(), Self::Error>;
    fn pad(&mut self, until: u64) -> Result<(), Self::Error>;
    fn align(&mut self, multiple_of: u64) -> Result<(), Self::Error>;

    fn deserialize_composite<O>(
        &mut self,
        deserialize_members: impl FnOnce(&mut Self::CompositeDeserializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error>;

    fn with_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        deserialize_members: impl FnOnce(&mut Self::ByteOrderDeserializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error>;
}
