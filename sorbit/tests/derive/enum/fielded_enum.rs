use crate::utility::{from_bytes, to_bytes};
use rstest::rstest;
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum Enum {
    #[sorbit(len=3, byte_order=little_endian)]
    A(u16) = 0x20,
    #[sorbit(byte_order=big_endian)]
    B { b: u32 } = 0x21,
    #[sorbit(catch_all)]
    CatchAll(u8) = 0x22,
}

#[rstest]
#[case(Enum::A(0x1234), &[0x20, 0x34, 0x12, 0x00])]
#[case(Enum::B{b: 0x5678ABCD}, &[0x21, 0x56, 0x78, 0xAB, 0xCD])]
#[case(Enum::CatchAll(0x93), &[0x93])]
fn serialize(#[case] value: Enum, #[case] bytes: &[u8]) {
    assert_eq!(to_bytes(&value), Ok(bytes.into()));
    assert_eq!(from_bytes::<Enum>(&bytes), Ok(value));
}
