use core::ops::Range;

use crate::serialize::Serialize;

pub trait Serializer: Sized {
    type Error;
    type StructSerializer: StructSerializer<Error = Self::Error>;
    type BitFieldSerializer: BitFieldSerializer<Error = Self::Error>;

    fn serialize_bool(self, value: u8) -> Result<(), Self::Error>;
    fn serialize_u8(self, value: u8) -> Result<(), Self::Error>;
    fn serialize_u16(self, value: u16) -> Result<(), Self::Error>;
    fn serialize_u32(self, value: u32) -> Result<(), Self::Error>;
    fn serialize_u64(self, value: u64) -> Result<(), Self::Error>;
    fn serialize_i8(self, value: u8) -> Result<(), Self::Error>;
    fn serialize_i16(self, value: u16) -> Result<(), Self::Error>;
    fn serialize_i32(self, value: u32) -> Result<(), Self::Error>;
    fn serialize_i64(self, value: u64) -> Result<(), Self::Error>;
    fn serialize_array<const N: usize>(self, value: &[u8; N]) -> Result<(), Self::Error>;
    fn serialize_slice(self, value: &[u8]) -> Result<(), Self::Error>;

    fn nest(&mut self, serialize_members: impl FnOnce(Self) -> Result<(), Self::Error>);

    fn serialize_sequence<Item>(self, items: impl Iterator<Item = Item>) -> Result<(), Self::Error>
    where
        Item: Serialize;

    fn serialize_struct<MemberSerializer>(
        self,
        serialize_members: MemberSerializer,
    ) -> Result<(), Self::Error>
    where
        MemberSerializer: FnOnce(Self::StructSerializer) -> Result<(), Self::Error>;

    fn serialize_bit_field<MemberSerializer>(
        &self,
        serialize_members: MemberSerializer,
    ) -> Result<(), Self::Error>
    where
        MemberSerializer: FnOnce(Self::BitFieldSerializer) -> Result<(), Self::Error>;
}

pub trait StructSerializer {
    type Error;
    type BitFieldSerializer: BitFieldSerializer<Error = Self::Error>;

    fn serialize_member<T: Serialize>(
        &self,
        value: T,
        name: Option<&'static str>,
    ) -> Result<(), Self::Error>;

    fn serialize_bit_field<MemberSerializer>(
        &self,
        serialize_members: MemberSerializer,
    ) -> Result<(), Self::Error>
    where
        MemberSerializer: FnOnce(Self::BitFieldSerializer) -> Result<(), Self::Error>;
}

pub trait BitFieldSerializer {
    type Error;

    fn serialize_member<T: Serialize>(
        &self,
        value: T,
        offset: Range<u8>,
        name: Option<&'static str>,
    ) -> Result<(), Self::Error>;
}

pub trait BitCompress {
    type Error;
    fn compress(&self) -> Result<u64, Self::Error>;
}

pub trait BitDecompress: Sized {
    type Error;
    fn decompress(bits: u64) -> Result<Self, Self::Error>;
}
