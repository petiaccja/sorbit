use crate::utility::{from_bytes, to_bytes};
use rstest::rstest;
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum Enum {
    A,
    B = 0x21,
    C,
    D = 0x87,
}

#[rstest]
#[case(Enum::A, [0x00_u8])]
#[case(Enum::B, [0x21_u8])]
#[case(Enum::C, [0x22_u8])]
#[case(Enum::D, [0x87_u8])]
fn serialize(#[case] value: Enum, #[case] bytes: [u8; 1]) {
    assert_eq!(to_bytes(&value), Ok(bytes.into()));
}

#[rstest]
#[case(Enum::A, [0x00_u8])]
#[case(Enum::B, [0x21_u8])]
#[case(Enum::C, [0x22_u8])]
#[case(Enum::D, [0x87_u8])]
fn deserialize(#[case] value: Enum, #[case] bytes: [u8; 1]) {
    assert_eq!(from_bytes::<Enum>(&bytes), Ok(value));
}

#[test]
#[should_panic]
fn deserialize_invalid() {
    from_bytes::<Enum>(&[0xFF]).unwrap();
}
