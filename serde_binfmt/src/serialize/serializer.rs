use crate::byte_order::ByteOrder;

pub trait Serializer: Sized {
    type Error;
    type Nested: Serializer<Error = Self::Error>;

    fn serialize_bool(&mut self, value: bool) -> Result<(), Self::Error>;
    fn serialize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn serialize_u16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn serialize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn serialize_i8(&mut self, value: i8) -> Result<(), Self::Error>;
    fn serialize_i16(&mut self, value: i16) -> Result<(), Self::Error>;
    fn serialize_i32(&mut self, value: i32) -> Result<(), Self::Error>;
    fn serialize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), Self::Error>;
    fn serialize_slice(&mut self, value: &[u8]) -> Result<(), Self::Error>;
    fn serialize_composite(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>;
    fn change_byte_order(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>;
    fn pad(&mut self, until: u64) -> Result<(), Self::Error>;
    fn align(&mut self, multiple_of: u64) -> Result<(), Self::Error>;
}
