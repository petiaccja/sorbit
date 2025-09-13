use super::traits::{Read, Seek, SeekFrom, Write};
use crate::error::Error;

pub struct FixedByteStream<'buffer> {
    buffer: &'buffer mut [u8],
    pos: usize,
}

impl<'buffer> FixedByteStream<'buffer> {
    pub fn new(buffer: &'buffer mut [u8]) -> Self {
        Self { buffer, pos: 0 }
    }
}

impl<'buffer> Read for FixedByteStream<'buffer> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        if self.pos + bytes.len() <= self.buffer.len() {
            let range = self.pos..(self.pos + bytes.len());
            bytes.copy_from_slice(&self.buffer[range]);
            self.pos += bytes.len();
            Ok(())
        } else {
            Err(Error::EndOfFile)
        }
    }
}

impl<'buffer> Write for FixedByteStream<'buffer> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.pos + bytes.len() <= self.buffer.len() {
            let range = self.pos..(self.pos + bytes.len());
            self.buffer[range].copy_from_slice(bytes);
            self.pos += bytes.len();
            Ok(())
        } else {
            Err(Error::EndOfFile)
        }
    }
}

impl<'buffer> Seek for FixedByteStream<'buffer> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => usize::try_from(offset).map_err(|_| Error::EndOfFile),
            SeekFrom::End(offset) => {
                let pos = (self.buffer.len() as i64) + offset;
                usize::try_from(pos).map_err(|_| Error::EndOfFile)
            }
            SeekFrom::Current(offset) => {
                let pos = (self.pos as i64) + offset;
                usize::try_from(pos).map_err(|_| Error::EndOfFile)
            }
        }?;
        if (0..=self.buffer.len()).contains(&new_pos) {
            self.pos = new_pos;
            Ok(self.pos as u64)
        } else {
            Err(Error::EndOfFile)
        }
    }

    fn stream_position(&mut self) -> Result<u64, Error> {
        Ok(self.pos as u64)
    }

    fn stream_len(&mut self) -> Result<u64, Error> {
        Ok(self.buffer.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_created() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.stream_len(), Ok(7));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn read_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        let mut values = [0u8; 3];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(values, [1, 2, 3]);
        Ok(())
    }

    #[test]
    fn read_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        let mut values = [0u8; 7];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(values, [1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn read_outside_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        let mut values = [0u8; 8];
        assert_eq!(stream.read(&mut values), Err(Error::EndOfFile));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn write_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        let values = [0u8; 3];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(buffer, [0, 0, 0, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        let values = [0u8; 7];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(buffer, [0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn write_outside_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        let values = [0u8; 8];
        assert_eq!(stream.write(&values), Err(Error::EndOfFile));
        assert_eq!(stream.stream_position(), Ok(0));
        assert_eq!(buffer, [1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn seek_from_start_within_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Start(4)), Ok(4));
        assert_eq!(stream.pos, 4);
    }

    #[test]
    fn seek_from_start_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Start(9)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }

    #[test]
    fn seek_from_current_within_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Current(5)), Ok(5));
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Ok(3));
        assert_eq!(stream.pos, 3);
    }

    #[test]
    fn seek_from_current_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Current(9)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }

    #[test]
    fn seek_from_current_negative_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }

    #[test]
    fn seek_from_end_within_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::End(-3)), Ok(4));
        assert_eq!(stream.pos, 4);
    }

    #[test]
    fn seek_from_end_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::End(2)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }

    #[test]
    fn seek_from_end_negative_out_of_bounds() {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = FixedByteStream::new(&mut buffer);
        assert_eq!(stream.seek(SeekFrom::End(-12)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }
}
