//! Serialization traits and serializers.

mod serialize;
mod serializer;
mod stream_serializer;

pub use serialize::{DeferredSerialize, Serialize};
pub use serializer::{DeferredSerializer, Lookback, Serializer, SerializerOutput, Span};
pub use stream_serializer::StreamSerializer;
