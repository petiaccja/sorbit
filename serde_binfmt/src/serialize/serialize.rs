use crate::serialize::DeferredSerializer;

use super::Serializer;

pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error>;
}

pub trait DeferredSerialize {
    fn serialize<S: DeferredSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error>;
}
