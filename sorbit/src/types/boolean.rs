use crate::ser_de::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for bool {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.serialize_bool(*self)
    }
}

impl Deserialize for bool {
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.deserialize_bool()
    }
}
