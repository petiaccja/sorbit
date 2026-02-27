use crate::error::Error;

/// This trait allows for writing bytes into a sink.
///
/// This trait is used by some [`crate::ser_de::Deserializer`]s
/// that can deserialize from a plain byte stream.
pub trait Read {
    /// Read exactly as many bytes as fits in `bytes`.
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error>;
}

/// This trait allows for reading bytes from a source.
///
/// This trait is used by some [`crate::ser_de::Serializer`]s
/// that can serialize into a plain byte stream.
pub trait Write {
    /// Write exactly as many bytes as there are in `bytes`.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;
}

/// Enumeration of possible methods to seek within an I/O object.
///
/// Use by the [`Seek`] trait. Mimics [`std::io::Seek`], see its documentation
/// for more information.
pub enum SeekFrom {
    /// Seek this many bytes from the beginning of the stream.
    Start(u64),
    /// Seek this many bytes from the end of the stream.
    End(i64),
    /// Seek this many bytes from the current stream position.
    Current(i64),
}

/// The [`Seek`]` trait provides a cursor which can be moved within a stream of bytes.
///
/// This trait is necessary because this crate is `no_std`, but the [`std::io`]
/// traits aren't available in `core`. This trait mimics [`std::io::Seek`], see
/// its documentation for more information.
pub trait Seek {
    /// Seek to a byte offset in the stream. The seek mode and amount is given by `pos`.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error>;

    /// Seek the stream to its beginning.
    fn rewind(&mut self) -> Result<(), Error> {
        self.seek(SeekFrom::Start(0)).map(|_| ())
    }

    /// Return the length of the stream.
    ///
    /// Unless optimized by the stream, it will attempt to seek to the end of
    /// the stream, read the position, and then seek back to the original
    /// position. A failure this might leave the stream's current position
    /// changed.
    fn stream_len(&mut self) -> Result<u64, Error> {
        let original_pos = self.stream_position()?;
        let end_pos = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(original_pos))?;
        Ok(end_pos)
    }

    /// Return the read/write cursor's current byte offset.
    fn stream_position(&mut self) -> Result<u64, Error> {
        self.seek(SeekFrom::Current(0))
    }

    /// Seek relative to the current stream position.
    fn seek_relative(&mut self, offset: i64) -> Result<(), Error> {
        self.seek(SeekFrom::Current(offset)).map(|_| ())
    }
}

impl SeekFrom {
    /// Given the current stream position and stream length, determine where
    /// the cursor will would be after this seek.
    pub fn absolute(&self, stream_pos: u64, stream_len: u64) -> i64 {
        match self {
            SeekFrom::Start(offset) => *offset as i64,
            SeekFrom::End(offset) => (stream_len as i64) + offset,
            SeekFrom::Current(offset) => (stream_pos as i64) + offset,
        }
    }
}

#[cfg(feature = "std")]
impl From<SeekFrom> for std::io::SeekFrom {
    fn from(value: SeekFrom) -> Self {
        match value {
            SeekFrom::Start(offset) => std::io::SeekFrom::Start(offset),
            SeekFrom::End(offset) => std::io::SeekFrom::End(offset),
            SeekFrom::Current(offset) => std::io::SeekFrom::Current(offset),
        }
    }
}

impl<T: Read + ?Sized> Read for &mut T {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        (**self).read(bytes)
    }
}

impl<T: Write + ?Sized> Write for &mut T {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        (**self).write(bytes)
    }
}

impl<T: Seek + ?Sized> Seek for &mut T {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        (**self).seek(pos)
    }

    fn rewind(&mut self) -> Result<(), Error> {
        (**self).rewind()
    }

    fn stream_len(&mut self) -> Result<u64, Error> {
        (**self).stream_len()
    }

    fn stream_position(&mut self) -> Result<u64, Error> {
        (**self).stream_position()
    }

    fn seek_relative(&mut self, offset: i64) -> Result<(), Error> {
        (**self).seek_relative(offset)
    }
}
