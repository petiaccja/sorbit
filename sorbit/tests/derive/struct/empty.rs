use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Empty {}

const LEN_VALUE: Empty = Empty {};
const LEN_BYTES: [u8; 0] = [];

#[test]
fn serialize() {
    assert_eq!(to_bytes(&LEN_VALUE), Ok(LEN_BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(from_bytes::<Empty>(&LEN_BYTES), Ok(LEN_VALUE));
}
