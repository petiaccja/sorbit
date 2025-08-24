use crate::serializer::Serializer;

pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<(), S::Error>;
}
