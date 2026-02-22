use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order = big_endian)]
struct BitNumberLSB0 {
    #[sorbit(bit_field=_b, repr=u8, bit_numbering = LSB0)]
    #[sorbit(bit_field=_b, bits=0..4)]
    a: u8,
    #[sorbit(bit_field=_b, bits=4..8)]
    b: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order = big_endian)]
struct BitNumberMSB0 {
    #[sorbit(bit_field=_b, repr=u8, bit_numbering = MSB0)]
    #[sorbit(bit_field=_b, bits=4..8)]
    a: u8,
    #[sorbit(bit_field=_b, bits=0..4)]
    b: u8,
}

const LSB0_VALUE: BitNumberLSB0 = BitNumberLSB0 { a: 0b1010, b: 0b1010 };
const MSB0_VALUE: BitNumberMSB0 = BitNumberMSB0 { a: 0b1010, b: 0b1010 };
const BYTES: [u8; 1] = [0b10101010];

#[test]
fn serialize_lsb0() {
    assert_eq!(to_bytes(&LSB0_VALUE), Ok(BYTES.into()));
}

#[test]
fn deserialize_lsb0() {
    assert_eq!(from_bytes::<BitNumberLSB0>(&BYTES), Ok(LSB0_VALUE));
}

#[test]
fn serialize_msb0() {
    assert_eq!(to_bytes(&MSB0_VALUE), Ok(BYTES.into()));
}

#[test]
fn deserialize_msb0() {
    assert_eq!(from_bytes::<BitNumberMSB0>(&BYTES), Ok(MSB0_VALUE));
}
