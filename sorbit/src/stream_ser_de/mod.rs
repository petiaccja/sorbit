//! A serializer and a deserializer that works with any stream.

mod stream_deserializer;
mod stream_serializer;

pub use stream_deserializer::StreamDeserializer;
pub use stream_serializer::StreamSerializer;
