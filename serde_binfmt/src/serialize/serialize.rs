use super::Serializer;

pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}
