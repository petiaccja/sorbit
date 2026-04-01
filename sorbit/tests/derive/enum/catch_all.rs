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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum CatchAllStruct {
    A,
    #[sorbit(catch_all)]
    CatchAll {
        catch_all: u8,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum CatchAllTupleContent {
    A,
    #[sorbit(catch_all)]
    CatchAll(u8, #[sorbit(byte_order = big_endian)] u16),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
enum CatchAllStructContent {
    A,
    #[sorbit(catch_all)]
    CatchAll {
        catch_all: u8,
        #[sorbit(byte_order = big_endian)]
        content: u16,
    },
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

#[rstest]
#[case(CatchAllStruct::A, [0x00_u8])]
#[case(CatchAllStruct::CatchAll{ catch_all: 0x01 }, [0x01_u8])]
#[case(CatchAllStruct::CatchAll{ catch_all: 0x87 }, [0x87_u8])]
fn deserialize_struct(#[case] value: CatchAllStruct, #[case] bytes: [u8; 1]) {
    assert_eq!(from_bytes::<CatchAllStruct>(&bytes), Ok(value));
}

#[rstest]
#[case(CatchAllTupleContent::A, [0x00_u8])]
#[case(CatchAllTupleContent::CatchAll(0x01, 0xABCD), [0x01_u8, 0xAB, 0xCD])]
#[case(CatchAllTupleContent::CatchAll(0x87, 0xABCD), [0x87_u8, 0xAB, 0xCD])]
fn deserialize_tuple_content(#[case] value: CatchAllTupleContent, #[case] bytes: impl AsRef<[u8]>) {
    assert_eq!(from_bytes::<CatchAllTupleContent>(bytes.as_ref()), Ok(value));
}

#[rstest]
#[case(CatchAllStructContent::A, [0x00_u8])]
#[case(CatchAllStructContent::CatchAll{ catch_all: 0x01, content: 0xABCD}, [0x01_u8, 0xAB, 0xCD])]
#[case(CatchAllStructContent::CatchAll{ catch_all: 0x87, content: 0xABCD}, [0x87_u8, 0xAB, 0xCD])]
fn deserialize_struct_content(#[case] value: CatchAllStructContent, #[case] bytes: impl AsRef<[u8]>) {
    assert_eq!(from_bytes::<CatchAllStructContent>(bytes.as_ref()), Ok(value));
}
