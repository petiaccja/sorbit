use crate::error::Error;

/// This trait allows for writing bytes into a sink.
///
/// This trait is used by some [`crate::deserialize::Deserializer`]s
/// that can deserialize from a plain byte stream.
pub trait Read {
    /// Read exactly as many bytes as fits in `bytes`.
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error>;
}

/// This trait allows for reading bytes from a source.
///
/// This trait is used by some [`crate::serialize::Serializer`]s
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
    Start(u64),
    End(i64),
    Current(i64),
}

/// The [`Seek`]` trait provides a cursor which can be moved within a stream of bytes.
///
/// This trait is necessary because this crate is `no_std`, but the [`std::io`]
/// traits aren't available in `core`. This trait mimics [`std::io::Seek`], see
/// its documentation for more information.
pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error>;

    fn rewind(&mut self) -> Result<(), Error> {
        self.seek(SeekFrom::Start(0)).map(|_| ())
    }

    fn stream_len(&mut self) -> Result<u64, Error> {
        let original_pos = self.stream_position()?;
        let end_pos = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(original_pos))?;
        Ok(end_pos)
    }

    fn stream_position(&mut self) -> Result<u64, Error> {
        self.seek(SeekFrom::Current(0))
    }

    fn seek_relative(&mut self, offset: i64) -> Result<(), Error> {
        self.seek(SeekFrom::Current(offset)).map(|_| ())
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

#[cfg(feature = "std")]
impl<T: std::io::Read> Read for T {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        <Self as std::io::Read>::read_exact(self, bytes).map_err(|err| err.into())
    }
}

#[cfg(feature = "std")]
impl<T: std::io::Write> Write for T {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        <Self as std::io::Write>::write_all(self, bytes).map_err(|err| err.into())
    }
}

#[cfg(feature = "std")]
impl<T: std::io::Seek> Seek for T {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        <Self as std::io::Seek>::seek(self, pos.into()).map_err(|err| err.into())
    }

    fn stream_position(&mut self) -> Result<u64, Error> {
        <Self as std::io::Seek>::stream_position(self).map_err(|err| err.into())
    }
}
