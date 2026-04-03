use crate::error::MessageError;
use crate::ser_de::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for char {
    /// Serialize the character as a 4-byte code point.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.serialize_u32((*self).into())
    }
}

impl Deserialize for char {
    /// Deserialize the character as a 4-byte code point.
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_u32()
            .map(|x| char::try_from(x).map_err(|_| D::Error::message("invalid code point")))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use crate::ser_de::{FromBytes, ToBytes};

    #[test]
    pub fn serialize_valid_char() {
        let value = 'A';
        let bytes = u32::from(value).to_be_bytes();
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), bytes);
        assert_eq!(<char as FromBytes>::from_be_bytes(&bytes).unwrap(), value);
    }

    #[test]
    pub fn serialize_invalid_char() {
        let bytes = [0x00, 0x11, 0x00, 0x00];
        assert!(<char as FromBytes>::from_be_bytes(&bytes).is_err());
    }
}
