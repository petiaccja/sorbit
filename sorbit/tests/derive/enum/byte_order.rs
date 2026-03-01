use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u16)]
#[sorbit(byte_order=big_endian)]
enum BigEndian {
    A = 0xFF00,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[repr(u16)]
#[sorbit(byte_order=little_endian)]
enum LittleEndian {
    A = 0xFF00,
}

#[test]
fn serialize_be() {
    assert_eq!(to_bytes(&BigEndian::A), Ok([0xFF, 0x00].into()));
}

#[test]
fn deserialize_be() {
    assert_eq!(from_bytes::<BigEndian>(&[0xFF, 0x00]), Ok(BigEndian::A));
}

#[test]
fn serialize_le() {
    assert_eq!(to_bytes(&LittleEndian::A), Ok([0x00, 0xFF].into()));
}

#[test]
fn deserialize_le() {
    assert_eq!(from_bytes::<LittleEndian>(&[0x00, 0xFF]), Ok(LittleEndian::A));
}
