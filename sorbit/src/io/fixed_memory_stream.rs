use super::stream::{Read, Seek, SeekFrom, Write};
use crate::error::{Error, ErrorKind};

/// A stream with an in-memory buffer that has a fixed size.
///
/// You may pass a vector, an in-memory slice, or a memory mapped file, mutable or not.
/// The size of the buffer will never be changed, even if the type you passed is
/// resizable. Reads and writes outside the buffer will result in an error.
#[derive(Debug)]
pub struct FixedMemoryStream<Buffer> {
    buffer: Buffer,
    stream_pos: usize,
}

impl<Buffer> FixedMemoryStream<Buffer> {
    /// Create a stream from the given buffer.
    pub fn new(buffer: Buffer) -> Self {
        Self { buffer, stream_pos: 0 }
    }
}

impl<Buffer: AsRef<[u8]>> Read for FixedMemoryStream<Buffer> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        if self.stream_pos + bytes.len() <= self.buffer.as_ref().len() {
            let range = self.stream_pos..(self.stream_pos + bytes.len());
            bytes.copy_from_slice(&self.buffer.as_ref()[range]);
            self.stream_pos += bytes.len();
            Ok(())
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl<Buffer: AsMut<[u8]>> Write for FixedMemoryStream<Buffer> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.stream_pos + bytes.len() <= self.buffer.as_mut().len() {
            let range = self.stream_pos..(self.stream_pos + bytes.len());
            self.buffer.as_mut()[range].copy_from_slice(bytes);
            self.stream_pos += bytes.len();
            Ok(())
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl<Buffer: AsRef<[u8]>> Seek for FixedMemoryStream<Buffer> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        let new_stream_pos = pos.absolute(self.stream_pos as u64, self.buffer.as_ref().len() as u64);
        let seek_range = 0..=(self.buffer.as_ref().len() as i64);
        if seek_range.contains(&new_stream_pos) {
            self.stream_pos = new_stream_pos as usize;
            Ok(self.stream_pos as u64)
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }

    fn stream_position(&mut self) -> Result<u64, Error> {
        Ok(self.stream_pos as u64)
    }

    fn stream_len(&mut self) -> Result<u64, Error> {
        Ok(self.buffer.as_ref().len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_created() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.stream_len(), Ok(7));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn read_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        let mut values = [0u8; 3];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(values, [1, 2, 3]);
        Ok(())
    }

    #[test]
    fn read_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        let mut values = [0u8; 7];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(values, [1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn read_outside_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        let mut values = [0u8; 8];
        assert_eq!(stream.read(&mut values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn write_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        let values = [0u8; 3];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(buffer, [0, 0, 0, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        let values = [0u8; 7];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(buffer, [0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn write_outside_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        let values = [0u8; 8];
        assert_eq!(stream.write(&values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        assert_eq!(buffer, [1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn seek_from_start_within_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Start(4)), Ok(4));
        assert_eq!(stream.stream_pos, 4);
    }

    #[test]
    fn seek_from_start_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Start(9)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }

    #[test]
    fn seek_from_current_within_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Current(5)), Ok(5));
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Ok(3));
        assert_eq!(stream.stream_pos, 3);
    }

    #[test]
    fn seek_from_current_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Current(9)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }

    #[test]
    fn seek_from_current_negative_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }

    #[test]
    fn seek_from_end_within_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::End(-3)), Ok(4));
        assert_eq!(stream.stream_pos, 4);
    }

    #[test]
    fn seek_from_end_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::End(2)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }

    #[test]
    fn seek_from_end_negative_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedMemoryStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::End(-12)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }
}
