//! Serialization traits and serializers.

mod serialize;
mod serializer;
mod stream_serializer;

pub use serialize::{MultiPassSerialize, Serialize};
pub use serializer::{Lookback, MultiPassSerializer, Serializer, SerializerOutput, Span};
pub use stream_serializer::StreamSerializer;
