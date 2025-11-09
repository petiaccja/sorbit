use sorbit::error::Error;
use sorbit::io::GrowingMemoryStream;
use sorbit::serialize::{Serialize, StreamSerializer};
use sorbit::{Deserialize, Serialize};

mod bit_fields;
mod generics;

#[derive(Debug, Serialize, Deserialize)]
struct Empty {}

#[derive(Debug, Serialize, Deserialize)]
struct WithoutLayout {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[sorbit_layout(len = 12)]
struct WithLen {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[sorbit_layout(round = 8)]
struct WithRounding {
    a: [u8; 10],
}

#[derive(Debug, Serialize, Deserialize)]
struct WithDirectFields {
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
fn serialize_with_len() -> Result<(), Error> {
    let input = WithLen { a: 83, b: 8 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![83, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    Ok(())
}

#[test]
fn serialize_with_rounding() -> Result<(), Error> {
    let input = WithRounding { a: [3; 10] };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0]);
    Ok(())
}

#[test]
fn serialize_without_layout() -> Result<(), Error> {
    let input = WithoutLayout { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x03, 0x12]);
    Ok(())
}

#[test]
fn serialize_direct_fields() -> Result<(), Error> {
    let input = WithDirectFields { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x00, 0x00, 0x03, 0x00, 0x12, 0x00]);
    Ok(())
}
