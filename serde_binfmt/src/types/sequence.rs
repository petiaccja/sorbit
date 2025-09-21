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
