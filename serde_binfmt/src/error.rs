#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    LengthExceedsPadding,
    EndOfFile,
    Unknown,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Error::LengthExceedsPadding => "The current length of the buffer already exceeds the requested padding",
            Error::EndOfFile => "End of file reached, cannot read/write more data",
            Error::Unknown => "An unknown error occured",
        };
        f.write_str(message)
    }
}

impl core::error::Error for Error {}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(_value: std::io::Error) -> Self {
        Self::Unknown
    }
}
