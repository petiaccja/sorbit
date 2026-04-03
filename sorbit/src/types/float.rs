use crate::ser_de::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for f32 {
    /// Serialize the floating point object.
    ///
    /// The floating point type is first converted to its raw bits using
    /// [`to_bits`](f32::to_bits), then serialized as an integer using the
    /// current byte order.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.serialize_u32(self.to_bits())
    }
}

impl Deserialize for f32 {
    /// Deserialize a floating point object.
    ///
    /// The bits of the floating point object are first deserialized as an
    /// integer, and then converted to a float using [`from_bits`](f32::from_bits).
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.deserialize_u32().map(|bits| f32::from_bits(bits))
    }
}

impl Serialize for f64 {
    /// Serialize the floating point object.
    ///
    /// The floating point type is first converted to its raw bits using
    /// [`to_bits`](f64::to_bits), then serialized as an integer using the
    /// current byte order.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.serialize_u64(self.to_bits())
    }
}

impl Deserialize for f64 {
    /// Deserialize a floating point object.
    ///
    /// The bits of the floating point object are first deserialized as an
    /// integer, and then converted to a float using [`from_bits`](f64::from_bits).
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.deserialize_u64().map(|bits| f64::from_bits(bits))
    }
}

#[cfg(test)]
mod tests {
    use crate::ser_de::{FromBytes, ToBytes};

    use rstest::rstest;

    #[rstest]
    #[case(0.9345)]
    #[case(f32::INFINITY)]
    #[case(f32::NEG_INFINITY)]
    pub fn serialize_isize(#[case] value: f32) {
        let bytes = value.to_be_bytes();
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), bytes);
        assert_eq!(<f32 as FromBytes>::from_be_bytes(&bytes).unwrap(), value);
    }

    #[rstest]
    #[case(0.9345)]
    #[case(f64::INFINITY)]
    #[case(f64::NEG_INFINITY)]
    pub fn serialize_usize(#[case] value: f64) {
        let bytes = value.to_be_bytes();
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), bytes);
        assert_eq!(<f64 as FromBytes>::from_be_bytes(&bytes).unwrap(), value);
    }
}
