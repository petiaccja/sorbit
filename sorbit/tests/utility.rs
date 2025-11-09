use sorbit::deserialize::{Deserialize, StreamDeserializer};
use sorbit::error::Error;
use sorbit::io::GrowingMemoryStream;
use sorbit::serialize::{Serialize, StreamSerializer};

pub fn to_bytes<Value: Serialize>(value: &Value) -> Result<Vec<u8>, Error> {
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    value.serialize(&mut serializer)?;
    Ok(serializer.take().take())
}

pub fn from_bytes<Value: Deserialize>(bytes: &[u8]) -> Result<Value, Error> {
    let mut deserializer = StreamDeserializer::new(GrowingMemoryStream::from(bytes));
    Value::deserialize(&mut deserializer)
}
