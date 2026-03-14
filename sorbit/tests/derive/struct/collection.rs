use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByLength {
    #[sorbit(value=len(collection))]
    len: u16,
    collection: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByLengthBit {
    #[sorbit(bit_field=_0, repr=u8, bits=0..=4)]
    #[sorbit(value=len(collection))]
    len: u8,
    collection: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByLengthOverflow {
    #[sorbit(bit_field=_0, repr=u8, bits=0..=1)]
    #[sorbit(value=len(collection))]
    len: u8,
    collection: Vec<u8>,
}

fn by_length_value(synchronize_len: bool) -> ByLength {
    ByLength { len: if synchronize_len { 4 } else { 0 }, collection: vec![1, 2, 3, 4] }
}
const BY_LENGTH_BYTES: [u8; 6] = [0, 4, 1, 2, 3, 4];

fn by_length_bit_value(synchronize_len: bool) -> ByLengthBit {
    ByLengthBit { len: if synchronize_len { 4 } else { 0 }, collection: vec![1, 2, 3, 4] }
}
const BY_LENGTH_BIT_BYTES: [u8; 5] = [4, 1, 2, 3, 4];

fn by_length_overflow_value(synchronize_len: bool) -> ByLengthOverflow {
    ByLengthOverflow { len: if synchronize_len { 4 } else { 0 }, collection: vec![1, 2, 3, 4] }
}

#[test]
fn serialize() {
    assert_eq!(to_bytes(&by_length_value(false)), Ok(BY_LENGTH_BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(from_bytes::<ByLength>(&BY_LENGTH_BYTES), Ok(by_length_value(true)));
}

#[test]
fn serialize_bit() {
    assert_eq!(to_bytes(&by_length_bit_value(false)), Ok(BY_LENGTH_BIT_BYTES.into()));
}

#[test]
fn deserialize_bit() {
    assert_eq!(from_bytes::<ByLengthBit>(&BY_LENGTH_BIT_BYTES), Ok(by_length_bit_value(true)));
}

#[test]
fn serialize_overflow() {
    assert!(to_bytes(&by_length_overflow_value(false)).is_err());
}
