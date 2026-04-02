use std::marker::PhantomData;

use sorbit::bit::{PackInto, UnpackFrom};
use sorbit::ser_de::{Deserialize, FromBytes as _, Serialize, ToBytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order = big_endian)]
struct Generic<T: Serialize + Deserialize + PartialEq> {
    value: T,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order = big_endian)]
struct GenericStructField {
    field: GenericValue<()>,
    #[sorbit(bit_field=_0, repr=u8, bits=0..8)]
    bit_field: GenericValue<()>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order = big_endian)]
#[repr(u8)]
enum GenericEnumField {
    Variant {
        field: GenericValue<()>,
        #[sorbit(bit_field=_0, repr=u8, bits=0..8)]
        bit_field: GenericValue<()>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct GenericValue<T> {
    value: u8,
    _type: PhantomData<T>,
}

impl<T, Packed> PackInto<Packed> for GenericValue<T>
where
    u8: PackInto<Packed>,
{
    fn pack_into(&self, num_bits: usize) -> Option<Packed> {
        self.value.pack_into(num_bits)
    }
}

impl<T, Packed> UnpackFrom<Packed> for GenericValue<T>
where
    u8: UnpackFrom<Packed>,
{
    fn unpack_from(value: Packed, num_bits: usize) -> Result<Self, Packed> {
        Ok(Self { value: u8::unpack_from(value, num_bits)?, _type: PhantomData })
    }
}

const VALUE: Generic<i32> = Generic::<i32> { value: -72 };
const BYTES: [u8; 4] = (-72i32).cast_unsigned().to_be_bytes();

const VALUE_STRUCT_FIELD: GenericStructField = GenericStructField {
    field: GenericValue { value: 0xAB, _type: PhantomData },
    bit_field: GenericValue { value: 0xCD, _type: PhantomData },
};
const BYTES_STRUCT_FIELD: [u8; 2] = [0xAB, 0xCD];

const VALUE_ENUM_FIELD: GenericEnumField = GenericEnumField::Variant {
    field: GenericValue { value: 0xAB, _type: PhantomData },
    bit_field: GenericValue { value: 0xCD, _type: PhantomData },
};
const BYTES_ENUM_FIELD: [u8; 3] = [0x00, 0xAB, 0xCD];

#[test]
fn serialize() {
    assert_eq!(VALUE.to_bytes(), Ok(BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(Generic::<i32>::from_bytes(&BYTES), Ok(VALUE));
}

#[test]
fn serialize_struct_field() {
    assert_eq!(VALUE_STRUCT_FIELD.to_bytes(), Ok(BYTES_STRUCT_FIELD.into()));
}

#[test]
fn deserialize_struct_field() {
    assert_eq!(GenericStructField::from_bytes(&BYTES_STRUCT_FIELD), Ok(VALUE_STRUCT_FIELD));
}

#[test]
fn serialize_enum_field() {
    assert_eq!(VALUE_ENUM_FIELD.to_bytes(), Ok(BYTES_ENUM_FIELD.into()));
}

#[test]
fn deserialize_enum_field() {
    assert_eq!(GenericEnumField::from_bytes(&BYTES_ENUM_FIELD), Ok(VALUE_ENUM_FIELD));
}
