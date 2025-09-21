use crate::byte_order::ByteOrder;
use crate::io::{Read, Seek};

pub trait Serializer: Sized {
    type Ok;
    type Error;
    type Nested: Serializer<Ok = Self::Ok, Error = Self::Error>;

    fn serialize_bool(&mut self, value: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_u8(&mut self, value: u8) -> Result<Self::Ok, Self::Error>;
    fn serialize_u16(&mut self, value: u16) -> Result<Self::Ok, Self::Error>;
    fn serialize_u32(&mut self, value: u32) -> Result<Self::Ok, Self::Error>;
    fn serialize_u64(&mut self, value: u64) -> Result<Self::Ok, Self::Error>;
    fn serialize_i8(&mut self, value: i8) -> Result<Self::Ok, Self::Error>;
    fn serialize_i16(&mut self, value: i16) -> Result<Self::Ok, Self::Error>;
    fn serialize_i32(&mut self, value: i32) -> Result<Self::Ok, Self::Error>;
    fn serialize_i64(&mut self, value: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<Self::Ok, Self::Error>;
    fn serialize_slice(&mut self, value: &[u8]) -> Result<Self::Ok, Self::Error>;
    fn pad(&mut self, until: u64) -> Result<Self::Ok, Self::Error>;
    fn align(&mut self, multiple_of: u64) -> Result<Self::Ok, Self::Error>;

    fn serialize_composite<O>(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<O, Self::Error>,
    ) -> Result<Self::Ok, Self::Error>;

    fn change_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<O, Self::Error>,
    ) -> Result<Self::Ok, Self::Error>;
}

pub trait Section {
    fn len(&self) -> u64;
    fn start(&self) -> u64;
    fn end(&self) -> u64;
}

pub trait DeferredSerializer: Serializer<Ok: Section> {
    type SectionSerializer: Serializer<Ok = Self::Ok, Error = Self::Error>;
    type SectionReader: Read + Seek;

    fn update_section<O>(
        &mut self,
        section: &Self::Ok,
        update_section: impl FnOnce(&mut Self::SectionSerializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error>;

    fn read_section<Output>(
        &mut self,
        section: &Self::Ok,
        analyze_bytes: impl FnOnce(&mut Self::SectionReader) -> Output,
    ) -> Result<Output, Self::Error>;
}
