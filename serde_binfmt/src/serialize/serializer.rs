use crate::byte_order::ByteOrder;
use crate::io::{Read, Seek};

pub trait Section {
    fn len(&self) -> u64;
    fn start(&self) -> u64;
    fn end(&self) -> u64;
}

pub trait SerializerOutput {
    type Success;
    type Error;
}

pub trait Serializer: SerializerOutput {
    type CompositeSerializer: Serializer<Success = Self::Success, Error = Self::Error>;
    type ByteOrderSerializer: Serializer<Success = Self::Success, Error = Self::Error>;

    fn serialize_bool(&mut self, value: bool) -> Result<Self::Success, Self::Error>;
    fn serialize_u8(&mut self, value: u8) -> Result<Self::Success, Self::Error>;
    fn serialize_u16(&mut self, value: u16) -> Result<Self::Success, Self::Error>;
    fn serialize_u32(&mut self, value: u32) -> Result<Self::Success, Self::Error>;
    fn serialize_u64(&mut self, value: u64) -> Result<Self::Success, Self::Error>;
    fn serialize_i8(&mut self, value: i8) -> Result<Self::Success, Self::Error>;
    fn serialize_i16(&mut self, value: i16) -> Result<Self::Success, Self::Error>;
    fn serialize_i32(&mut self, value: i32) -> Result<Self::Success, Self::Error>;
    fn serialize_i64(&mut self, value: i64) -> Result<Self::Success, Self::Error>;
    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<Self::Success, Self::Error>;
    fn serialize_slice(&mut self, value: &[u8]) -> Result<Self::Success, Self::Error>;
    fn pad(&mut self, until: u64) -> Result<Self::Success, Self::Error>;
    fn align(&mut self, multiple_of: u64) -> Result<Self::Success, Self::Error>;
    fn set_byte_order(&mut self, byte_order: ByteOrder) -> ByteOrder;
    fn get_byte_order(&self) -> ByteOrder;

    fn serialize_composite<O>(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::CompositeSerializer) -> Result<O, Self::Error>,
    ) -> Result<Self::Success, Self::Error>;

    fn with_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self::ByteOrderSerializer) -> Result<O, Self::Error>,
    ) -> Result<Self::Success, Self::Error>;
}

pub trait Lookback: SerializerOutput {
    type SectionSerializer: Serializer + Lookback<Success = Self::Success, Error = Self::Error>;
    type SectionReader: Read + Seek;

    fn update_section<O>(
        &mut self,
        section: &Self::Success,
        update_section: impl FnOnce(&mut Self::SectionSerializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error>;

    fn analyze_section<Output>(
        &mut self,
        section: &Self::Success,
        analyze_bytes: impl FnOnce(&mut Self::SectionReader) -> Output,
    ) -> Result<Output, Self::Error>;
}

pub trait DeferredSerializer:
    Serializer<CompositeSerializer: Lookback, ByteOrderSerializer: Lookback> + Lookback
{
}

impl<S> DeferredSerializer for S where
    S: Serializer<CompositeSerializer: Lookback, ByteOrderSerializer: Lookback> + Lookback
{
}
