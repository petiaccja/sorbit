//! I/O traits and I/O streams.

mod basic_stream;
mod fixed_memory_stream;
#[cfg(feature = "alloc")]
mod growing_memory_stream;
mod partial_stream;

pub use basic_stream::{Read, Seek, SeekFrom, Write};
pub use fixed_memory_stream::FixedMemoryStream;
#[cfg(feature = "alloc")]
pub use growing_memory_stream::GrowingMemoryStream;
pub use partial_stream::StreamSection;
