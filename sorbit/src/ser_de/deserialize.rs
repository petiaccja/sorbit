use crate::ser_de::Deserializer;

/// The type can be deserialized from a [`Deserializer`].
///
/// `Deserialize` is implemented by sorbit for primitive types. For your own
/// types, in many cases, you can achieve the desired layout using the derive
/// macro together with sorbit's layout control attributes. In some cases though,
/// you will need to implement `Deserialize` yourself to get the desired layout.
pub trait Deserialize
where
    Self: Sized,
{
    /// Try to deserialize this object from the `deserializer`.
    ///
    /// Deserialization might fail in case, for example, the bit representation
    /// is incorrect, or if an end of file is encountered.
    ///
    /// In case of a failure, it's up to the `deserializer` to roll back partial
    /// changes or to enter an indeterminate state.
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error>;
}
