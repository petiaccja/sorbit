use crate::serialize::Serialize;

impl Serialize for u8 {
    fn serialize<S: crate::serializer::Serializer>(&self, serializer: S) -> Result<(), S::Error> {
        serializer.serialize_u8(*self)
    }
}

impl Serialize for u16 {
    fn serialize<S: crate::serializer::Serializer>(&self, serializer: S) -> Result<(), S::Error> {
        serializer.serialize_u16(*self)
    }
}
