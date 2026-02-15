use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NoLayout {
    #[sorbit(byte_order=little_endian)]
    le: u16,
    #[sorbit(byte_order=big_endian)]
    be: u16,
    #[sorbit(bit_field=_le, repr=u16, bits=0..16, byte_order=little_endian)]
    le_bit: u16,
    #[sorbit(bit_field=_be, repr=u16, bits=0..16, byte_order=big_endian)]
    be_bit: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WithLayout {
    #[sorbit(byte_order=little_endian, align=1, round=1)]
    le: u16,
    #[sorbit(byte_order=big_endian, align=1, round=1)]
    be: u16,
    #[sorbit(bit_field=_le, repr=u16, bits=0..16, byte_order=little_endian, align=1, round=1)]
    le_bit: u16,
    #[sorbit(bit_field=_be, repr=u16, bits=0..16, byte_order=big_endian, align=1, round=1)]
    be_bit: u16,
}

const NO_LAYOUT_VALUE: NoLayout = NoLayout { le: 0xFF00, be: 0xFF00, le_bit: 0xFF00, be_bit: 0xFF00 };
const NO_LAYOUT_BYTES: [u8; 8] = [0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00];

const WITH_LAYOUT_VALUE: WithLayout = WithLayout { le: 0xFF00, be: 0xFF00, le_bit: 0xFF00, be_bit: 0xFF00 };
const WITH_LAYOUT_BYTES: [u8; 8] = [0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00];

#[test]
fn serialize_no_layout() {
    assert_eq!(to_bytes(&NO_LAYOUT_VALUE), Ok(NO_LAYOUT_BYTES.into()));
}

#[test]
fn deserialize_no_layout() {
    assert_eq!(from_bytes::<NoLayout>(&NO_LAYOUT_BYTES), Ok(NO_LAYOUT_VALUE));
}

#[test]
fn serialize_with_layout() {
    assert_eq!(to_bytes(&WITH_LAYOUT_VALUE), Ok(WITH_LAYOUT_BYTES.into()));
}

#[test]
fn deserialize_with_layout() {
    assert_eq!(from_bytes::<WithLayout>(&WITH_LAYOUT_BYTES), Ok(WITH_LAYOUT_VALUE));
}
