use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Constant {
    #[sorbit(value = constant(13))]
    c1: u8,
    #[sorbit(bit_field = _0, repr = u8, bits = 0..8, value = constant(14u8))]
    c2: u8,
}

const LEN_VALUE_NO_SYNC: Constant = Constant { c1: 0, c2: 0 };
const LEN_VALUE_SYNC: Constant = Constant { c1: 13, c2: 14 };
const LEN_BYTES: [u8; 2] = [13, 14];

#[test]
fn serialize() {
    assert_eq!(to_bytes(&LEN_VALUE_NO_SYNC), Ok(LEN_BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(from_bytes::<Constant>(&LEN_BYTES), Ok(LEN_VALUE_SYNC));
}

#[test]
fn deserialize_wrong() {
    assert!(from_bytes::<Constant>(&[43, 28]).is_err());
}
