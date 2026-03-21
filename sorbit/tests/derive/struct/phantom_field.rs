use std::marker::PhantomData;

use sorbit::{Deserialize, Serialize, ser_de::FromBytes, ser_de::ToBytes};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Plain {
    data: PhantomData<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Constant {
    #[sorbit(value = constant(13u8))]
    data: PhantomData<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Len {
    #[sorbit(value = len(collection))]
    data: PhantomData<u8>,
    collection: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ByteCount {
    #[sorbit(value = byte_count(collection))]
    data: PhantomData<u8>,
    collection: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ConstantBit {
    #[sorbit(bit_field=_b0, repr=u8, bits=0..8)]
    #[sorbit(value = constant(13u8))]
    data: PhantomData<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct LenBit {
    #[sorbit(bit_field=_b0, repr=u8, bits=0..8)]
    #[sorbit(value = len(collection))]
    data: PhantomData<u8>,
    collection: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ByteCountBit {
    #[sorbit(bit_field=_b0, repr=u8, bits=0..8)]
    #[sorbit(value = byte_count(collection))]
    data: PhantomData<u8>,
    collection: Vec<u8>,
}

const PLAIN_VALUE: Plain = Plain { data: PhantomData };
const PLAIN_BYTES: [u8; 0] = [];

const CONSTANT_VALUE: Constant = Constant { data: PhantomData };
const CONSTANT_BYTES: [u8; 1] = [13];

const LEN_VALUE: Len = Len { data: PhantomData, collection: Vec::new() };
const LEN_BYTES: [u8; 1] = [0];

const BYTE_COUNT_VALUE: ByteCount = ByteCount { data: PhantomData, collection: Vec::new() };
const BYTE_COUNT_BYTES: [u8; 1] = [0];

const CONSTANT_VALUE_BIT: ConstantBit = ConstantBit { data: PhantomData };
const CONSTANT_BYTES_BIT: [u8; 1] = [13];

const LEN_VALUE_BIT: LenBit = LenBit { data: PhantomData, collection: Vec::new() };
const LEN_BYTES_BIT: [u8; 1] = [0];

const BYTE_COUNT_VALUE_BIT: ByteCountBit = ByteCountBit { data: PhantomData, collection: Vec::new() };
const BYTE_COUNT_BYTES_BIT: [u8; 1] = [0];

#[test]
fn serialize_plain() {
    assert_eq!(PLAIN_VALUE.to_bytes(), Ok(PLAIN_BYTES.into()));
}

#[test]
fn deserialize_plain() {
    assert_eq!(Plain::from_bytes(&PLAIN_BYTES), Ok(PLAIN_VALUE));
}

#[test]
fn serialize_constant() {
    assert_eq!(CONSTANT_VALUE.to_bytes(), Ok(CONSTANT_BYTES.into()));
}

#[test]
fn deserialize_constant() {
    assert_eq!(Constant::from_bytes(&CONSTANT_BYTES), Ok(CONSTANT_VALUE));
}

#[test]
fn serialize_len() {
    assert_eq!(LEN_VALUE.to_bytes(), Ok(LEN_BYTES.into()));
}

#[test]
fn deserialize_len() {
    assert_eq!(Len::from_bytes(&LEN_BYTES), Ok(LEN_VALUE));
}

#[test]
fn serialize_byte_count() {
    assert_eq!(BYTE_COUNT_VALUE.to_bytes(), Ok(BYTE_COUNT_BYTES.into()));
}

#[test]
fn deserialize_byte_count() {
    assert_eq!(ByteCount::from_bytes(&BYTE_COUNT_BYTES), Ok(BYTE_COUNT_VALUE));
}

#[test]
fn serialize_constant_bit() {
    assert_eq!(CONSTANT_VALUE_BIT.to_bytes(), Ok(CONSTANT_BYTES_BIT.into()));
}

#[test]
fn deserialize_constant_bit() {
    assert_eq!(ConstantBit::from_bytes(&CONSTANT_BYTES_BIT), Ok(CONSTANT_VALUE_BIT));
}

#[test]
fn serialize_len_bit() {
    assert_eq!(LEN_VALUE_BIT.to_bytes(), Ok(LEN_BYTES_BIT.into()));
}

#[test]
fn deserialize_len_bit() {
    assert_eq!(LenBit::from_bytes(&LEN_BYTES_BIT), Ok(LEN_VALUE_BIT));
}

#[test]
fn serialize_byte_count_bit() {
    assert_eq!(BYTE_COUNT_VALUE_BIT.to_bytes(), Ok(BYTE_COUNT_BYTES_BIT.into()));
}

#[test]
fn deserialize_byte_count_bit() {
    assert_eq!(ByteCountBit::from_bytes(&BYTE_COUNT_BYTES_BIT), Ok(BYTE_COUNT_VALUE_BIT));
}
