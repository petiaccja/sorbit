use sorbit::error::Error;
use sorbit::io::GrowingMemoryStream;
use sorbit::ser_de::{Deserialize, Serialize};
use sorbit::stream_ser_de::{StreamDeserializer, StreamSerializer};

pub fn to_bytes<Value: Serialize>(value: &Value) -> Result<Vec<u8>, Error> {
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    value.serialize(&mut serializer)?;
    Ok(serializer.take().take())
}

pub fn from_bytes<Value: Deserialize>(bytes: &[u8]) -> Result<Value, Error> {
    let mut deserializer = StreamDeserializer::new(GrowingMemoryStream::from(bytes));
    Value::deserialize(&mut deserializer)
}
