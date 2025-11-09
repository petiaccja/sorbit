use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit_layout(len = 3)]
struct Len {
    a: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit_layout(round = 5)]
struct Round {
    a: u32,
}

const LEN_VALUE: Len = Len { a: 54 };
const LEN_BYTES: [u8; 3] = [54, 0, 0];

const ROUND_VALUE: Round = Round { a: 54 };
const ROUND_BYTES: [u8; 5] = [0, 0, 0, 54, 0];

#[test]
fn serialize_len() {
    assert_eq!(to_bytes(&LEN_VALUE), Ok(LEN_BYTES.into()));
}

#[test]
fn deserialize_len() {
    assert_eq!(from_bytes::<Len>(&LEN_BYTES), Ok(LEN_VALUE));
}

#[test]
fn serialize_round() {
    assert_eq!(to_bytes(&ROUND_VALUE), Ok(ROUND_BYTES.into()));
}

#[test]
fn deserialize_round() {
    assert_eq!(from_bytes::<Round>(&ROUND_BYTES), Ok(ROUND_VALUE));
}
