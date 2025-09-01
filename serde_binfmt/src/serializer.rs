pub trait Serializer: Sized {
    type Error;
    type Nested: Serializer<Error = Self::Error>;

    fn serialize_bool(&mut self, value: u8) -> Result<(), Self::Error>;
    fn serialize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn serialize_u16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn serialize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn serialize_i8(&mut self, value: u8) -> Result<(), Self::Error>;
    fn serialize_i16(&mut self, value: u16) -> Result<(), Self::Error>;
    fn serialize_i32(&mut self, value: u32) -> Result<(), Self::Error>;
    fn serialize_i64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<(), Self::Error>;
    fn serialize_slice(&mut self, value: &[u8]) -> Result<(), Self::Error>;
    fn composite(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>;
    fn pad(until: usize) -> Result<(), Self::Error>;
    fn align(alignment: usize) -> Result<(), Self::Error>;
}
