#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    LengthExceedsPadding,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Error::LengthExceedsPadding => "The current length of the buffer already exceeds the requested padding",
        };
        f.write_str(message)
    }
}
