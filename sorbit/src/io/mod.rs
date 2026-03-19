//! I/O traits and I/O streams.

mod bounded_section;
mod fixed_memory_stream;
#[cfg(feature = "alloc")]
mod growing_memory_stream;
mod stream;
mod stream_section;

pub use bounded_section::BoundedSection;
pub use fixed_memory_stream::FixedMemoryStream;
#[cfg(feature = "alloc")]
pub use growing_memory_stream::GrowingMemoryStream;
pub use stream::{Bounded, Read, Seek, SeekFrom, Write};
pub use stream_section::StreamSection;
