//! Serialization traits and serializers.

mod serialize;
mod serializer;
mod stream_serializer;

pub use serialize::{MultiPassSerialize, Serialize};
pub use serializer::{MultiPassSerializer, RevisableSerializer, SerializationOutcome, Serializer, Span};
pub use stream_serializer::StreamSerializer;
