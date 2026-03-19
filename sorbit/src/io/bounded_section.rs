use crate::error::{Error, ErrorKind};
use crate::io::{Bounded, Read, Write};

/// A wrapper around a stream that limits the amount of bytes that can be read
/// or written.
///
/// A bounded stream section can be used to bring the end of the stream closer
/// by limiting how much can be read from or written to it. Once the quota has
/// been exhausted, the bounded section returns EOF even if there is still data
/// in the underlying stream. This is useful when you want to limit the
/// read/write operations to only a part of a stream.
///
/// This is similar to a [`StreamSection`](crate::io::StreamSection), but it
/// does not need the stream to be seekable.
#[derive(Debug)]
pub struct BoundedSection<Stream> {
    stream: Stream,
    remaining_bytes: u64,
}

impl<Stream> BoundedSection<Stream> {
    /// Return the original stream.
    pub fn into_inner(self) -> Stream {
        self.stream
    }
}

impl<Stream> BoundedSection<Stream> {
    /// Create a bounded section by wrapping another stream.
    ///
    /// # Parameters
    ///
    /// - `stream`: the stream to wrap.
    /// - `num_bytes`: the number of bytes that may be read from the stream
    ///   starting from the current stream position.
    pub fn new(stream: Stream, num_bytes: u64) -> Self {
        Self { stream, remaining_bytes: num_bytes }
    }
}

impl<Stream: Read> Read for BoundedSection<Stream> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        let bytes_to_read = bytes.len() as u64;
        if bytes_to_read <= self.remaining_bytes {
            self.remaining_bytes -= bytes_to_read;
            self.stream.read(bytes)
        } else {
            self.remaining_bytes = 0;
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl<Stream: Write> Write for BoundedSection<Stream> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let bytes_to_write = bytes.len() as u64;
        if bytes_to_write <= self.remaining_bytes {
            self.remaining_bytes -= bytes_to_write;
            self.stream.write(bytes)
        } else {
            self.remaining_bytes = 0;
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl<Stream> Bounded for BoundedSection<Stream> {
    fn remaining_bytes(&self) -> u64 {
        self.remaining_bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::io::FixedMemoryStream;

    use super::*;

    #[test]
    fn newly_created() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        assert_eq!(stream.remaining_bytes(), 4);
        Ok(())
    }

    #[test]
    fn read_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        let mut values = [0u8; 3];
        stream.read(&mut values)?;
        assert_eq!(stream.remaining_bytes(), 1);
        assert_eq!(stream.is_finished(), false);
        assert_eq!(values, [1, 2, 3]);
        Ok(())
    }

    #[test]
    fn read_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        let mut values = [0u8; 4];
        stream.read(&mut values)?;
        assert_eq!(stream.remaining_bytes(), 0);
        assert_eq!(stream.is_finished(), true);
        assert_eq!(values, [1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn read_outside_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        let mut values = [0u8; 5];
        assert_eq!(stream.read(&mut values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.remaining_bytes(), 0);
        assert_eq!(stream.is_finished(), true);
        Ok(())
    }

    #[test]
    fn write_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        let values = [0u8; 3];
        stream.write(&values)?;
        assert_eq!(stream.remaining_bytes(), 1);
        assert_eq!(stream.is_finished(), false);
        assert_eq!(buffer, [0, 0, 0, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        let values = [0u8; 4];
        stream.write(&values)?;
        assert_eq!(stream.remaining_bytes(), 0);
        assert_eq!(stream.is_finished(), true);
        assert_eq!(buffer, [0, 0, 0, 0, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_outside_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = BoundedSection::new(FixedMemoryStream::new(&mut buffer), 4);
        let values = [0u8; 5];
        assert_eq!(stream.write(&values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.remaining_bytes(), 0);
        assert_eq!(stream.is_finished(), true);
        assert_eq!(buffer, [1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }
}
