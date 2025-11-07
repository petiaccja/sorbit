use crate::bit::Error as BitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorKind {
    LengthExceedsPadding,
    UnexpectedEof,
    InvalidEnumVariant,
    Bit(BitError),
    #[cfg(feature = "std")]
    IO(std::io::ErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Error {
    kind: ErrorKind,
    item: Item,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Item {
    #[cfg(not(feature = "alloc"))]
    name: Option<&'static str>,
    #[cfg(feature = "alloc")]
    path: Vec<String>,
}

pub trait SerializeError: Sized + From<BitError> {
    #[cfg(not(feature = "alloc"))]
    fn enclose(self, ident: &'static str) -> Self;

    #[cfg(feature = "alloc")]
    fn enclose(self, ident: &str) -> Self;
}

//------------------------------------------------------------------------------
// Error implementations
//------------------------------------------------------------------------------

impl From<BitError> for Error {
    fn from(value: BitError) -> Self {
        Self { kind: ErrorKind::Bit(value), item: Item::default() }
    }
}

impl SerializeError for Error {
    #[cfg(not(feature = "alloc"))]
    fn enclose(self, ident: &'static str) -> Self {
        Self { kind: self.kind, item: self.item.enclose(ident) }
    }

    #[cfg(feature = "alloc")]
    fn enclose(self, ident: &str) -> Self {
        Self { kind: self.kind, item: self.item.enclose(ident) }
    }
}

impl core::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if !self.item.is_empty() {
            write!(f, "{}: {}", self.item, self.kind)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self { kind: value, item: Item::default() }
    }
}

//------------------------------------------------------------------------------
// ErrorKind implementations
//------------------------------------------------------------------------------

impl core::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use ErrorKind::*;
        match self {
            LengthExceedsPadding => write!(f, "the current length of the buffer already exceeds the requested padding"),
            UnexpectedEof => write!(f, "end of file reached, cannot read/write more data"),
            InvalidEnumVariant => write!(f, "the numeric value does not correspond to an enum or bool variant"),
            Bit(err) => write!(f, "the bit field cannot be packed: {err}"),
            #[cfg(feature = "std")]
            IO(kind) => write!(f, "{kind}"),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for ErrorKind {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::UnexpectedEof => ErrorKind::UnexpectedEof,
            kind => ErrorKind::IO(kind),
        }
    }
}

//------------------------------------------------------------------------------
// Item implementations
//------------------------------------------------------------------------------

impl Item {
    #[cfg(not(feature = "alloc"))]
    pub fn is_empty(&self) -> bool {
        self.name.is_some()
    }

    #[cfg(feature = "alloc")]
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    #[cfg(not(feature = "alloc"))]
    pub fn enclose(self, ident: &'static str) -> Self {
        Self { name: Some(self.name.unwrap_or(ident)) }
    }

    #[cfg(feature = "alloc")]
    pub fn enclose(mut self, ident: &str) -> Self {
        self.path.push(ident.into());
        self
    }
}

impl core::fmt::Display for Item {
    #[cfg(not(feature = "alloc"))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.name {
            Some(name) => write!(f, "{name}"),
            None => Ok(()),
        }
    }

    #[cfg(feature = "alloc")]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.path.iter().rev().next().map(|root| write!(f, ".{root}")).unwrap_or(Ok(()))?;
        for ident in self.path.iter().rev().skip(1) {
            write!(f, ".{ident}")?
        }
        Ok(())
    }
}
