use crate::deserialize::Deserialize;
use crate::serialize::Serialize;
use crate::serialize::Serializer;

impl Serialize for [u8] {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.serialize_slice(self)
    }
}

impl<const N: usize> Serialize for [u8; N] {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.serialize_array(self)
    }
}

impl<const N: usize> Deserialize for [u8; N] {
    fn deserialize<D: crate::deserialize::Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.deserialize_array()
    }
}
