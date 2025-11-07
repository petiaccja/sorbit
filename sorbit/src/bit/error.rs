#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    TooManyBits,
    Overlap,
    OutOfRange,
    ReversedRange,
}

impl core::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::TooManyBits => write!(f, "could not fit a value into a the target type"),
            Error::Overlap => write!(f, "the field's bit range overlaps with fields previously packed"),
            Error::OutOfRange => write!(f, "the packed value's target bit range falls outside the packed type"),
            Error::ReversedRange => write!(f, "bit ranges must not be reversed"),
        }
    }
}
