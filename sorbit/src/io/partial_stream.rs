use core::ops::Range;

use crate::error::{Error, ErrorKind};
use crate::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct PartialStream<Stream> {
    stream: Stream,
    range: Range<u64>,
}

impl<Stream> PartialStream<Stream> {
    pub fn into_inner(self) -> Stream {
        self.stream
    }
}

impl<Stream: Seek> PartialStream<Stream> {
    pub fn new(mut stream: Stream, range: Range<u64>) -> Result<Self, Stream> {
        match stream.seek(SeekFrom::Start(range.start)) {
            Ok(_) => Ok(Self { stream, range }),
            Err(_) => Err(stream),
        }
    }
}

impl<Stream: Read + Seek> Read for PartialStream<Stream> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        let stream_pos = self.stream.stream_position()?;
        let read_range = stream_pos..(stream_pos + bytes.len() as u64);
        if range_contains(&self.range, &read_range) {
            self.stream.read(bytes)
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl<Stream: Write + Seek> Write for PartialStream<Stream> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let stream_pos = self.stream.stream_position()?;
        let write_range = stream_pos..(stream_pos + bytes.len() as u64);
        if range_contains(&self.range, &write_range) {
            self.stream.write(bytes)
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }
}

impl<Stream: Seek> Seek for PartialStream<Stream> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        let new_stream_pos = pos.absolute(self.stream_position()?, self.stream_len()?);
        let seek_range = 0..=(self.range.end - self.range.start) as i64;
        if seek_range.contains(&new_stream_pos) {
            let new_underlying_stream_pos = self.range.start + new_stream_pos as u64;
            self.stream.seek(SeekFrom::Start(new_underlying_stream_pos))?;
            Ok(new_stream_pos as u64)
        } else {
            Err(ErrorKind::UnexpectedEof.into())
        }
    }

    fn stream_len(&mut self) -> Result<u64, Error> {
        Ok(self.range.end - self.range.start)
    }

    fn stream_position(&mut self) -> Result<u64, Error> {
        let underlying_stream_pos = self.stream.stream_position()?;
        assert!(
            self.range.contains(&underlying_stream_pos) || self.range.end == underlying_stream_pos,
            "the underlying stream's position should never be allowed to go outside the valid range of the partial stream"
        );
        Ok(underlying_stream_pos - self.range.start)
    }
}

fn range_contains(outer: &Range<u64>, inner: &Range<u64>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
mod tests {
    use crate::io::FixedMemoryStream;

    use super::*;

    #[test]
    fn newly_created() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.stream_len(), Ok(4));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }

    #[test]
    fn read_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        let mut values = [0u8; 3];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(values, [3, 4, 5]);
        Ok(())
    }

    #[test]
    fn read_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        let mut values = [0u8; 4];
        stream.read(&mut values)?;
        assert_eq!(stream.stream_position(), Ok(4));
        assert_eq!(values, [3, 4, 5, 6]);
        Ok(())
    }

    #[test]
    fn read_outside_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        let mut values = [0u8; 5];
        assert_eq!(stream.read(&mut values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }

    #[test]
    fn write_well_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        let values = [0u8; 3];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(3));
        assert_eq!(buffer, [1, 2, 0, 0, 0, 6, 7]);
        Ok(())
    }

    #[test]
    fn write_just_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        let values = [0u8; 4];
        stream.write(&values)?;
        assert_eq!(stream.stream_position(), Ok(4));
        assert_eq!(buffer, [1, 2, 0, 0, 0, 0, 7]);
        Ok(())
    }

    #[test]
    fn write_outside_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        let values = [0u8; 5];
        assert_eq!(stream.write(&values), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        assert_eq!(buffer, [1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn seek_from_start_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::Start(3)), Ok(3));
        assert_eq!(stream.stream_position(), Ok(3));
        Ok(())
    }

    #[test]
    fn seek_from_start_out_of_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::Start(5)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }

    #[test]
    fn seek_from_current_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::Current(3)), Ok(3));
        assert_eq!(stream.seek(SeekFrom::Current(-1)), Ok(2));
        assert_eq!(stream.stream_position(), Ok(2));
        Ok(())
    }

    #[test]
    fn seek_from_current_out_of_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::Current(5)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }

    #[test]
    fn seek_from_current_negative_out_of_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::Current(-2)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }

    #[test]
    fn seek_from_end_within_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::End(-3)), Ok(1));
        assert_eq!(stream.stream_position(), Ok(1));
        Ok(())
    }

    #[test]
    fn seek_from_end_out_of_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::End(2)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }

    #[test]
    fn seek_from_end_negative_out_of_bounds() -> Result<(), Error> {
        let mut buffer = [1, 2, 3, 4, 5, 6, 7];
        let mut stream = PartialStream::new(FixedMemoryStream::new(&mut buffer), 2..6).expect("new failed");
        assert_eq!(stream.seek(SeekFrom::End(-12)), Err(ErrorKind::UnexpectedEof.into()));
        assert_eq!(stream.stream_position(), Ok(0));
        Ok(())
    }
}
