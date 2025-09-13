use super::traits::{Read, Seek, SeekFrom, Write};
use crate::error::Error;
use alloc::vec::Vec;

pub struct ByteStream {
    buffer: Vec<u8>,
    pos: usize,
}

impl ByteStream {
    pub fn new() -> Self {
        Self { buffer: Vec::new(), pos: 0 }
    }

    pub fn take(self) -> Vec<u8> {
        self.buffer
    }
}

impl From<Vec<u8>> for ByteStream {
    fn from(value: Vec<u8>) -> Self {
        Self { buffer: value, pos: 0 }
    }
}

impl From<&[u8]> for ByteStream {
    fn from(value: &[u8]) -> Self {
        Self { buffer: value.into(), pos: 0 }
    }
}

impl Read for ByteStream {
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

impl Write for ByteStream {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let new_len = core::cmp::max(self.buffer.len(), self.pos + bytes.len());
        self.buffer.resize(new_len, 0);
        let range = self.pos..(self.pos + bytes.len());
        self.buffer[range].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }
}

impl Seek for ByteStream {
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
        self.pos = new_pos;
        Ok(self.pos as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_created() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.stream_len(), Ok(7));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn read_well_within_bounds() -> Result<(), Error> {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let mut values = [0u8; 3];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(values, [1, 2, 3]);
        Ok(())
    }

    #[test]
    fn read_just_within_bounds() -> Result<(), Error> {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let mut values = [0u8; 7];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(values, [1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn read_outside_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let mut values = [0u8; 8];
        assert_eq!(stream.read(&mut values), Err(Error::EndOfFile));
        assert_eq!(stream.stream_position(), Ok(0));
    }

    #[test]
    fn write_well_within_bounds() -> Result<(), Error> {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [0u8; 3];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(stream.buffer, [0, 0, 0, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_just_within_bounds() -> Result<(), Error> {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [0u8; 7];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(7));
        assert_eq!(stream.buffer, [0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn write_partially_outside_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [0u8; 8];
        assert_eq!(stream.write(&values), Ok(()));
        assert_eq!(stream.stream_position(), Ok(8));
        assert_eq!(stream.buffer, [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn write_fully_outside_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        let values = [10u8; 2];
        stream.pos = 9;
        assert_eq!(stream.write(&values), Ok(()));
        assert_eq!(stream.stream_position(), Ok(11));
        assert_eq!(stream.buffer, [1, 2, 3, 4, 5, 6, 7, 0, 0, 10, 10]);
    }

    #[test]
    fn seek_from_start_within_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Start(4)), Ok(4));
        assert_eq!(stream.pos, 4);
    }

    #[test]
    fn seek_from_start_out_of_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Start(9)), Ok(9));
        assert_eq!(stream.pos, 9);
    }

    #[test]
    fn seek_from_current_within_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Current(5)), Ok(5));
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Ok(3));
        assert_eq!(stream.pos, 3);
    }

    #[test]
    fn seek_from_current_out_of_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Current(9)), Ok(9));
        assert_eq!(stream.pos, 9);
    }

    #[test]
    fn seek_from_current_negative_out_of_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }

    #[test]
    fn seek_from_end_within_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::End(-3)), Ok(4));
        assert_eq!(stream.pos, 4);
    }

    #[test]
    fn seek_from_end_out_of_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::End(2)), Ok(9));
        assert_eq!(stream.pos, 9);
    }

    #[test]
    fn seek_from_end_negative_out_of_bounds() {
        let mut stream = ByteStream::from(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(stream.seek(SeekFrom::End(-12)), Err(Error::EndOfFile));
        assert_eq!(stream.pos, 0);
    }
}
