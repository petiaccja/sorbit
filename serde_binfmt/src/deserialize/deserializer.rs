use crate::byte_order::ByteOrder;

pub trait Deserializer: Sized {
    type Error;
    type Nested: Deserializer<Error = Self::Error>;

    fn deserialize_bool(&mut self, value: bool) -> Result<(), Self::Error>;
    fn deserialize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn deserialize_u16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn deserialize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn deserialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn deserialize_i8(&mut self, value: i8) -> Result<(), Self::Error>;
    fn deserialize_i16(&mut self, value: i16) -> Result<(), Self::Error>;
    fn deserialize_i32(&mut self, value: i32) -> Result<(), Self::Error>;
    fn deserialize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn deserialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), Self::Error>;
    fn deserialize_slice(&mut self, value: &[u8]) -> Result<(), Self::Error>;
    fn deserialize_composite(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>;
    fn change_byte_order(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>;
    fn pad(&mut self, until: usize) -> Result<(), Self::Error>;
    fn align(&mut self, multiple_of: usize) -> Result<(), Self::Error>;
}
