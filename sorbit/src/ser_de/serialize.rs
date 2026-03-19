use crate::ser_de::{RevisableSerializer, Serializer};

/// The type can be serialized into a [`Serializer`].
///
/// `Serialize` is implemented by sorbit for primitive types. For your own
/// types, in many cases, you can achieve the desired layout using the derive
/// macro together with sorbit's layout control attributes. In some cases though,
/// you will need to implement `Serialize` yourself to get the desired layout.
pub trait Serialize {
    /// Try to serialize this object into the `serializer`.
    ///
    /// Serialization might fail in case, for example, a bit field member fails
    /// to pack into fewer bits, or if an end of file is encountered.
    ///
    /// In case of a failure, it's up to the `serializer` to roll back partial
    /// changes or to enter an indeterminate state.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error>;
}

/// The type requires multiple passes during serialization and can be
/// serialized only into a [`RevisableSerializer`].
///
/// This trait is analogous to [`Serialize`], but is meant for types that require
/// the extra features provided by multi-pass serializers. See [`Serialize`] for
/// more information.
pub trait MultiPassSerialize {
    /// Try to serialize this object into the `serializer`.
    ///
    /// See [`Serialize::serialize`] for more information.
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error>;
}

/// Blanket implementation of serialize for references.
impl<T: Serialize> Serialize for &T {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        (*self).serialize(serializer)
    }
}

/// Blanket implementation of multi-pass serialize for references.
impl<T: Serialize> MultiPassSerialize for &T {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        (*self).serialize(serializer)
    }
}
