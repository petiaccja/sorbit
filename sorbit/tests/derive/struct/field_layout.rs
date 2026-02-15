use crate::utility::{from_bytes, to_bytes};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Offset {
    pre: u8,
    #[sorbit(offset = 4)]
    subject: u8,
    post: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Align {
    pre: u8,
    #[sorbit(align = 4)]
    subject: u8,
    post: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Round {
    pre: u8,
    #[sorbit(round = 4)]
    subject: u8,
    post: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct All {
    pre: u8,
    #[sorbit(offset = 7, align = 2, round = 3)]
    subject: u8,
    post: u8,
}

const OFFSET_VALUE: Offset = Offset { pre: 0xFD, subject: 0xFE, post: 0xFF };
const OFFSET_BYTES: [u8; 6] = [0xFD, 0, 0, 0, 0xFE, 0xFF];

const ALIGN_VALUE: Align = Align { pre: 0xFD, subject: 0xFE, post: 0xFF };
const ALIGN_BYTES: [u8; 6] = [0xFD, 0, 0, 0, 0xFE, 0xFF];

const ROUND_VALUE: Round = Round { pre: 0xFD, subject: 0xFE, post: 0xFF };
const ROUND_BYTES: [u8; 6] = [0xFD, 0xFE, 0, 0, 0, 0xFF];

const ALL_VALUE: All = All { pre: 0xFD, subject: 0xFE, post: 0xFF };
const ALL_BYTES: [u8; 12] = [0xFD, 0, 0, 0, 0, 0, 0, 0, 0xFE, 0, 0, 0xFF];

#[test]
fn serialize_offset() {
    assert_eq!(to_bytes(&OFFSET_VALUE), Ok(OFFSET_BYTES.into()));
}

#[test]
fn deserialize_offset() {
    assert_eq!(from_bytes::<Offset>(&OFFSET_BYTES), Ok(OFFSET_VALUE));
}

#[test]
fn serialize_align() {
    assert_eq!(to_bytes(&ALIGN_VALUE), Ok(ALIGN_BYTES.into()));
}

#[test]
fn deserialize_align() {
    assert_eq!(from_bytes::<Align>(&ALIGN_BYTES), Ok(ALIGN_VALUE));
}

#[test]
fn serialize_round() {
    assert_eq!(to_bytes(&ROUND_VALUE), Ok(ROUND_BYTES.into()));
}

#[test]
fn deserialize_round() {
    assert_eq!(from_bytes::<Round>(&ROUND_BYTES), Ok(ROUND_VALUE));
}

#[test]
fn serialize_all() {
    assert_eq!(to_bytes(&ALL_VALUE), Ok(ALL_BYTES.into()));
}

#[test]
fn deserialize_all() {
    assert_eq!(from_bytes::<All>(&ALL_BYTES), Ok(ALL_VALUE));
}
