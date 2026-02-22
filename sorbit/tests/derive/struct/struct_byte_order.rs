use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=little_endian)]
struct LittleEndianOrder {
    field: u16,
    #[sorbit(bit_field=_be, repr=u16, bits=0..16)]
    bit_field: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct BigEndianOrder {
    field: u16,
    #[sorbit(bit_field=_be, repr=u16, bits=0..16)]
    bit_field: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NativeEndianOrder {
    field: u16,
    #[sorbit(bit_field=_be, repr=u16, bits=0..16)]
    bit_field: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=little_endian)]
struct Outer {
    value: u16,
    inner: Inner,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct Inner {
    value: u16,
}

const LITTLE_ENDIAN_VALUE: LittleEndianOrder = LittleEndianOrder { field: 0xFF00, bit_field: 0xFF00 };
const LITTLE_ENDIAN_BYTES: [u8; 4] = [0x00, 0xFF, 0x00, 0xFF];

const BIG_ENDIAN_VALUE: BigEndianOrder = BigEndianOrder { field: 0xFF00, bit_field: 0xFF00 };
const BIG_ENDIAN_BYTES: [u8; 4] = [0xFF, 0x00, 0xFF, 0x00];

const NATIVE_ENDIAN_VALUE: NativeEndianOrder = NativeEndianOrder { field: 0xFF00, bit_field: 0xFF00 };
const NATIVE_ENDIAN_BYTES: [u8; 4] = [
    0xFF00u16.to_ne_bytes()[0],
    0xFF00u16.to_ne_bytes()[1],
    0xFF00u16.to_ne_bytes()[0],
    0xFF00u16.to_ne_bytes()[1],
];

const NESTED_VALUE: Outer = Outer { value: 0xFF00, inner: Inner { value: 0xFF00 } };
const NESTED_BYTES: [u8; 4] = [0x00, 0xFF, 0xFF, 0x00];

#[test]
fn serialize_little() {
    assert_eq!(to_bytes(&LITTLE_ENDIAN_VALUE), Ok(LITTLE_ENDIAN_BYTES.into()));
}

#[test]
fn deserialize_little() {
    assert_eq!(from_bytes::<LittleEndianOrder>(&LITTLE_ENDIAN_BYTES), Ok(LITTLE_ENDIAN_VALUE));
}

#[test]
fn serialize_big() {
    assert_eq!(to_bytes(&BIG_ENDIAN_VALUE), Ok(BIG_ENDIAN_BYTES.into()));
}

#[test]
fn deserialize_big() {
    assert_eq!(from_bytes::<BigEndianOrder>(&BIG_ENDIAN_BYTES), Ok(BIG_ENDIAN_VALUE));
}

#[test]
fn serialize_native() {
    assert_eq!(to_bytes(&NATIVE_ENDIAN_VALUE), Ok(NATIVE_ENDIAN_BYTES.into()));
}

#[test]
fn deserialize_native() {
    assert_eq!(from_bytes::<NativeEndianOrder>(&NATIVE_ENDIAN_BYTES), Ok(NATIVE_ENDIAN_VALUE));
}

#[test]
fn serialize_nested() {
    assert_eq!(to_bytes(&NESTED_VALUE), Ok(NESTED_BYTES.into()));
}

#[test]
fn deserialize_nested() {
    assert_eq!(from_bytes::<Outer>(&NESTED_BYTES), Ok(NESTED_VALUE));
}
