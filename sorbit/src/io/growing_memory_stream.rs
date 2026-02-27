use super::stream::{Read, Seek, SeekFrom, Write};
use crate::error::{Error, ErrorKind};
use alloc::vec::Vec;

/// A stream with an in-memory buffer that grows on demand.
///
/// There is no limit on the maximum size of the memory stream.
/// It's essentially a [`Vec`] on which you keep pulling [`Vec::push`].
///
/// Keep in mind that you can seek past the size of the current size of the
/// internal buffer. Attempting to read the stream there will result in an error,
/// but writing the stream is valid and it will pad the buffer with zeros all the
/// way to the cursor. The memory usage will also experience a jump.
#[derive(Debug)]
pub struct GrowingMemoryStream {
    buffer: Vec<u8>,
    stream_pos: usize,
}

impl GrowingMemoryStream {
    /// Create a stream with a zero-sized buffer.
    pub fn new() -> Self {
        Self { buffer: Vec::new(), stream_pos: 0 }
    }

    /// Take the buffer of the stream.
    pub fn take(self) -> Vec<u8> {
        self.buffer
    }
}

impl From<Vec<u8>> for GrowingMemoryStream {
    fn from(value: Vec<u8>) -> Self {
        Self { buffer: value, stream_pos: 0 }
    }
}

impl From<&[u8]> for GrowingMemoryStream {
    fn from(value: &[u8]) -> Self {
        Self { buffer: value.into(), stream_pos: 0 }
    }
}

impl Read for GrowingMemoryStream {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        if self.stream_pos + bytes.len() <= self.buffer.len() {
            let range = self.stream_pos..(self.stream_pos + bytes.len());
            bytes.copy_from_slice(&self.buffer[range]);
            self.stream_pos += bytes.len();
            Ok(())
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl Write for GrowingMemoryStream {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let new_len = core::cmp::max(self.buffer.len(), self.stream_pos + bytes.len());
        self.buffer.resize(new_len, 0);
        let range = self.stream_pos..(self.stream_pos + bytes.len());
        self.buffer[range].copy_from_slice(bytes);
        self.stream_pos += bytes.len();
        Ok(())
    }
}

impl Seek for GrowingMemoryStream {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        let new_stream_pos = pos.absolute(self.stream_pos as u64, self.buffer.len() as u64);
        if let Ok(new_stream_pos) = usize::try_from(new_stream_pos) {
            self.stream_pos = new_stream_pos as usize;
            Ok(self.stream_pos as u64)
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_created() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.stream_len(), Ok(7));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn read_well_within_bounds() -> Result<(), Error> {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let mut values = [0u8; 3];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(values, [1, 2, 3]);
        Ok(())
    }

    #[test]
    fn read_just_within_bounds() -> Result<(), Error> {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let mut values = [0u8; 7];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(values, [1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn read_outside_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let mut values = [0u8; 8];
        assert_eq!(stream.read(&mut values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn write_well_within_bounds() -> Result<(), Error> {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [0u8; 3];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(stream.buffer, [0, 0, 0, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_just_within_bounds() -> Result<(), Error> {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [0u8; 7];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(stream.buffer, [0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn write_partially_outside_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [0u8; 8];
        assert_eq!(stream.write(&values), Ok(()));
        assert_eq!(stream.stream_position(), Ok(8));
        assert_eq!(stream.buffer, [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn write_fully_outside_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [10u8; 2];
        stream.stream_pos = 9;
        assert_eq!(stream.write(&values), Ok(()));
        assert_eq!(stream.stream_position(), Ok(11));
        assert_eq!(stream.buffer, [1, 2, 3, 4, 5, 6, 7, 0, 0, 10, 10]);
    }

    #[test]
    fn seek_from_start_within_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Start(4)), Ok(4));
        assert_eq!(stream.stream_pos, 4);
    }

    #[test]
    fn seek_from_start_out_of_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Start(9)), Ok(9));
        assert_eq!(stream.stream_pos, 9);
    }

    #[test]
    fn seek_from_current_within_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Current(5)), Ok(5));
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Ok(3));
        assert_eq!(stream.stream_pos, 3);
    }

    #[test]
    fn seek_from_current_out_of_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Current(9)), Ok(9));
        assert_eq!(stream.stream_pos, 9);
    }

    #[test]
    fn seek_from_current_negative_out_of_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }

    #[test]
    fn seek_from_end_within_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::End(-3)), Ok(4));
        assert_eq!(stream.stream_pos, 4);
    }

    #[test]
    fn seek_from_end_out_of_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::End(2)), Ok(9));
        assert_eq!(stream.stream_pos, 9);
    }

    #[test]
    fn seek_from_end_negative_out_of_bounds() {
        let mut stream = GrowingMemoryStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::End(-12)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_pos, 0);
    }
}
