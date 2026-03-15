//! Traits for serialization and deserialization.

mod byte_conv;
mod deserialize;
mod deserializer;
mod serialize;
mod serializer;

pub use byte_conv::{FromBytes, ToBytes};
pub use deserialize::Deserialize;
pub use deserializer::Deserializer;
pub use serialize::{MultiPassSerialize, Serialize};
pub use serializer::{MultiPassSerializer, RevisableSerializer, SerializationOutcome, Serializer, Span};
