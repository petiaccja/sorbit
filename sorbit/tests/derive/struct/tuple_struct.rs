use crate::utility::{from_bytes, to_bytes};
use sorbit::deserialize::Deserialize;
use sorbit::serialize::Serialize;
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Direct<T: Serialize + Deserialize + PartialEq>(T);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct BitField(#[sorbit(bit_field=_b, repr=u8, bits=0)] bool, #[sorbit(bit_field=_b, bits=1)] bool);

const DIRECT_VALUE: Direct<i32> = Direct::<i32>(-72);
const DIRECT_BYTES: [u8; 4] = (-72i32).cast_unsigned().to_be_bytes();

const BIT_FIELD_VALUE: BitField = BitField(false, true);
const BIT_FIELD_BYTES: [u8; 1] = [0b10];

#[test]
fn serialize_direct() {
    assert_eq!(to_bytes(&DIRECT_VALUE), Ok(DIRECT_BYTES.into()));
}

#[test]
fn deserialize_direct() {
    assert_eq!(from_bytes::<Direct<i32>>(&DIRECT_BYTES), Ok(DIRECT_VALUE));
}

#[test]
fn serialize_bit_field() {
    assert_eq!(to_bytes(&BIT_FIELD_VALUE), Ok(BIT_FIELD_BYTES.into()));
}

#[test]
fn deserialize_bit_field() {
    assert_eq!(from_bytes::<BitField>(&BIT_FIELD_BYTES), Ok(BIT_FIELD_VALUE));
}
