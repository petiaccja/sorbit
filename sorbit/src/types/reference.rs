use crate::ser_de::{MultiPassSerialize, RevisableSerializer, Serialize, Serializer};

/// Blanket implementation of serialize for references.
impl<T: Serialize> Serialize for &T {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        (*self).serialize(serializer)
    }
}

/// Blanket implementation of multi-pass serialize for references.
impl<T: MultiPassSerialize> MultiPassSerialize for &T {
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        (*self).serialize(serializer)
    }
}

/// Blanket implementation of serialize for mutable references.
impl<T: Serialize> Serialize for &mut T {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        (*self as &T).serialize(serializer)
    }
}

/// Blanket implementation of multi-pass serialize for mutable references.
impl<T: MultiPassSerialize> MultiPassSerialize for &mut T {
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        (*self as &T).serialize(serializer)
    }
}
