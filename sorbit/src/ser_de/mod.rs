//! Traits for serialization and deserialization.

mod deserialize;
mod deserializer;
mod serialize;
mod serializer;

pub use deserialize::Deserialize;
pub use deserializer::Deserializer;
pub use serialize::{MultiPassSerialize, Serialize};
pub use serializer::{MultiPassSerializer, RevisableSerializer, SerializationOutcome, Serializer, Span};
