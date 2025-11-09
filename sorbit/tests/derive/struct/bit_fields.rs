use crate::utility::{from_bytes, to_bytes};
use sorbit::{
    Deserialize, Serialize,
    bit::Error as BitError,
    error::{Error, SerializeError},
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit_bit_field(_b, repr(u16))]
struct Packing {
    #[sorbit_bit_field(_b, bits(4..10))]
    a: u8,
    #[sorbit_bit_field(_b, bits(14..=15))]
    b: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Layout {
    #[sorbit_bit_field(_b, repr(u16), offset = 2, round = 4, bits(4..10))]
    a: u8,
}

const PACKING_VALUE: Packing = Packing { a: 0b110011, b: true };
const PACKING_BYTES: [u8; 2] = 0b0100_0011_0011_0000_u16.to_be_bytes();

const LAYOUT_VALUE: Layout = Layout { a: 0b110011 };
const LAYOUT_BYTES: [u8; 6] = [0u8, 0u8, 0b0000_0011_u8, 0b0011_0000_u8, 0u8, 0u8];

#[test]
fn serialize_packing() {
    assert_eq!(to_bytes(&PACKING_VALUE), Ok(PACKING_BYTES.into()));
}

#[test]
fn deserialize_packing() {
    assert_eq!(from_bytes::<Packing>(&PACKING_BYTES), Ok(PACKING_VALUE));
}

#[test]
fn serialize_packing_bit_overflow() {
    let faulty_value = Packing { a: 255, b: true };
    assert_eq!(to_bytes(&faulty_value), Err(Error::from(BitError::TooManyBits).enclose("a")));
}

#[test]
fn deserialize_packing_invalid_variant() {
    let faulty_bytes = 0b1000_0011_0011_0000_u16.to_be_bytes();
    // TODO: this is not quite right, it should return an InvalidVariant or
    // similar error as `2` does not qualify as either `true` or `false`.
    assert_eq!(from_bytes::<Packing>(&faulty_bytes), Err(Error::from(BitError::TooManyBits).enclose("b")));
}

#[test]
fn serialize_layout() {
    assert_eq!(to_bytes(&LAYOUT_VALUE), Ok(LAYOUT_BYTES.into()));
}

#[test]
fn deserialize_layout() {
    assert_eq!(from_bytes::<Layout>(&LAYOUT_BYTES), Ok(LAYOUT_VALUE));
}
