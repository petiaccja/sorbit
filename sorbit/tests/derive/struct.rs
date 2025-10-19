#![allow(unused)]

use sorbit::error::Error;
use sorbit::io::GrowingMemoryStream;
use sorbit::serialize::{Serialize as _, StreamSerializer};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Empty {}

#[derive(Debug, Serialize, Deserialize)]
struct Unconstrained {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectFields {
    #[sorbit_layout(offset = 2)]
    a: u8,
    #[sorbit_layout(align = 4, round = 2)]
    b: u8,
}

#[test]
fn serialize_empty() -> Result<(), Error> {
    let input = Empty {};
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![]);
    Ok(())
}

#[test]
fn serialize_unconstrained() -> Result<(), Error> {
    let input = Unconstrained { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x03, 0x12]);
    Ok(())
}

#[test]
fn serialize_direct_fields() -> Result<(), Error> {
    let input = DirectFields { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x00, 0x00, 0x03, 0x00, 0x12, 0x00]);
    Ok(())
}
