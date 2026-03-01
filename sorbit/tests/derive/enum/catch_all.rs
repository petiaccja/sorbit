use crate::utility::{from_bytes, to_bytes};
use rstest::rstest;
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum CatchAllEmpty {
    A,
    #[sorbit(catch_all)]
    CatchAll,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum CatchAllTuple {
    A,
    #[sorbit(catch_all)]
    CatchAll(u8),
}

#[rstest]
#[case(CatchAllEmpty::A, [0x00_u8])]
#[case(CatchAllEmpty::CatchAll, [0x01_u8])]
fn serialize_empty(#[case] value: CatchAllEmpty, #[case] bytes: [u8; 1]) {
    assert_eq!(to_bytes(&value), Ok(bytes.into()));
}

#[rstest]
#[case(CatchAllEmpty::A, [0x00_u8])]
#[case(CatchAllEmpty::CatchAll, [0x01_u8])]
#[case(CatchAllEmpty::CatchAll, [0x87_u8])]
fn deserialize_empty(#[case] value: CatchAllEmpty, #[case] bytes: [u8; 1]) {
    assert_eq!(from_bytes::<CatchAllEmpty>(&bytes), Ok(value));
}

#[rstest]
#[case(CatchAllTuple::A, [0x00_u8])]
#[case(CatchAllTuple::CatchAll(0x01), [0x01_u8])]
#[case(CatchAllTuple::CatchAll(0x87), [0x87_u8])]
fn serialize_tuple(#[case] value: CatchAllTuple, #[case] bytes: [u8; 1]) {
    assert_eq!(to_bytes(&value), Ok(bytes.into()));
}

#[rstest]
#[case(CatchAllTuple::A, [0x00_u8])]
#[case(CatchAllTuple::CatchAll(0x01), [0x01_u8])]
#[case(CatchAllTuple::CatchAll(0x87), [0x87_u8])]
fn deserialize_tuple(#[case] value: CatchAllTuple, #[case] bytes: [u8; 1]) {
    assert_eq!(from_bytes::<CatchAllTuple>(&bytes), Ok(value));
}
