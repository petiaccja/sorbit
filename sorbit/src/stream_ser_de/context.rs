use core::ops::Range;

use crate::byte_order::ByteOrder;
use crate::error::{Error, ErrorKind};
use crate::io::{Read, Seek, Write};

#[derive(Debug, Clone)]
pub struct Context {
    /// The base address of the current composite.
    base_pos: u64,
    /// The position where the next read/write occurs.
    absolute_pos: u64,
    /// The byte order used to serialize items.
    byte_order: ByteOrder,
    /// Only bytes in range may be written or read.
    limits: Option<Range<u64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct CompositeScope {
    base_pos: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ByteOrderScope {
    byte_order: ByteOrder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BoundedScope {
    limits: Option<Range<u64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct RevisionScope {
    base_pos: u64,
    absolute_pos: u64,
    limits: Option<Range<u64>>,
}

impl Context {
    pub fn local_pos(&self) -> u64 {
        self.absolute_pos - self.base_pos
    }

    pub fn absolute_pos(&self) -> u64 {
        self.absolute_pos
    }

    pub fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }

    pub fn change_byte_order(self, byte_order: ByteOrder) -> Self {
        Self { byte_order, ..self }
    }

    pub fn bytes_in_bounds(&self) -> Option<u64> {
        self.limits.as_ref().map(|limits| limits.end - self.absolute_pos)
    }

    pub fn composite_scope(&mut self) -> CompositeScope {
        let base_pos = core::mem::replace(&mut self.base_pos, self.absolute_pos);
        CompositeScope { base_pos }
    }

    pub fn close_composite_scope(&mut self, scope: CompositeScope) {
        self.base_pos = scope.base_pos;
    }

    pub fn byte_order_scope(&mut self, byte_order: ByteOrder) -> ByteOrderScope {
        let byte_order = core::mem::replace(&mut self.byte_order, byte_order);
        ByteOrderScope { byte_order }
    }

    pub fn close_byte_order_scope(&mut self, scope: ByteOrderScope) {
        self.byte_order = scope.byte_order;
    }

    pub fn bounded_scope(&mut self, num_bytes: u64) -> Result<BoundedScope, Error> {
        let bounds = self.absolute_pos..self.absolute_pos + num_bytes;
        if self.limits.as_ref().is_some_and(|current| !contains_range(current, &bounds)) {
            Err(ErrorKind::OutOfBounds.into())
        } else {
            Ok(BoundedScope { limits: self.limits.replace(bounds) })
        }
    }

    pub fn close_bounded_scope(&mut self, scope: BoundedScope) {
        self.limits = scope.limits;
    }

    pub fn revision_scope(&mut self, stream: &mut impl Seek, span: Range<u64>) -> Result<RevisionScope, Error> {
        if self.limits.as_ref().is_some_and(|current| !contains_range(current, &span)) {
            Err(ErrorKind::OutOfBounds.into())
        } else {
            let relative_start = span.start as i64 - self.absolute_pos as i64;
            stream.seek_relative(relative_start)?;
            let absolute_pos = core::mem::replace(&mut self.absolute_pos, span.start);
            let base_pos = core::mem::replace(&mut self.base_pos, span.start);
            Ok(RevisionScope { base_pos, absolute_pos, limits: self.limits.replace(span) })
        }
    }

    pub fn close_revision_scope(&mut self, stream: &mut impl Seek, scope: RevisionScope) -> Result<(), Error> {
        let restore_offset = scope.absolute_pos as i64 - self.absolute_pos as i64;
        stream.seek_relative(restore_offset)?;
        self.base_pos = scope.base_pos;
        self.absolute_pos = scope.absolute_pos;
        self.limits = scope.limits;
        Ok(())
    }

    pub fn read(&mut self, stream: &mut impl Read, bytes: &mut [u8]) -> Result<Range<u64>, Error> {
        let read_span = self.absolute_pos..self.absolute_pos + bytes.len() as u64;
        if let Some(bounds) = &self.limits {
            if !contains_range(bounds, &read_span) {
                return Err(ErrorKind::OutOfBounds.into());
            };
        };
        match stream.read(bytes) {
            Ok(_) => {
                self.absolute_pos += bytes.len() as u64;
                Ok(read_span)
            }
            Err(err) => Err(err),
        }
    }

    pub fn write(&mut self, stream: &mut impl Write, bytes: &[u8]) -> Result<Range<u64>, Error> {
        let write_span = self.absolute_pos..self.absolute_pos + bytes.len() as u64;
        if let Some(bounds) = &self.limits {
            if !contains_range(bounds, &write_span) {
                return Err(ErrorKind::OutOfBounds.into());
            };
        };
        match stream.write(bytes) {
            Ok(_) => {
                self.absolute_pos += bytes.len() as u64;
                Ok(write_span)
            }
            Err(err) => Err(err),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self { base_pos: 0, absolute_pos: 0, byte_order: ByteOrder::native(), limits: None }
    }
}

fn contains_range(outer: &Range<u64>, inner: &Range<u64>) -> bool {
    outer.contains(&inner.start) && outer.contains(&(core::cmp::max(1, inner.end) - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::io::{GrowingMemoryStream, SeekFrom};

    #[test]
    fn test_contains_range() {
        assert_eq!(contains_range(&(3..6), &(3..6)), true);
        assert_eq!(contains_range(&(3..6), &(3..5)), true);
        assert_eq!(contains_range(&(3..6), &(3..7)), false);
        assert_eq!(contains_range(&(3..6), &(2..6)), false);
        assert_eq!(contains_range(&(3..6), &(2..7)), false);
    }

    #[test]
    fn composite_scope() {
        let mut ctx = Context::default();
        ctx.absolute_pos = 70;
        let scope = ctx.composite_scope();
        assert_eq!(ctx.base_pos, 70);
        ctx.absolute_pos += 20;
        ctx.close_composite_scope(scope);
        assert_eq!(ctx.base_pos, 0);
        assert_eq!(ctx.absolute_pos, 90);
    }

    #[test]
    fn byte_order_scope() {
        let mut ctx = Context::default();
        ctx.byte_order = ByteOrder::BigEndian;
        let scope = ctx.byte_order_scope(ByteOrder::LittleEndian);
        assert_eq!(ctx.byte_order, ByteOrder::LittleEndian);
        ctx.close_byte_order_scope(scope);
        assert_eq!(ctx.byte_order, ByteOrder::BigEndian);
    }

    #[test]
    fn bounded_scope_none() {
        let mut ctx = Context::default();
        ctx.absolute_pos = 70;

        let scope = ctx.bounded_scope(40).unwrap();
        assert_eq!(ctx.limits, Some(70..110));
        ctx.close_bounded_scope(scope);
        assert_eq!(ctx.limits, None);
    }

    #[test]
    fn bounded_scope_inside() {
        let mut ctx = Context::default();
        ctx.absolute_pos = 80;
        ctx.limits = Some(70..110);

        let scope = ctx.bounded_scope(20).unwrap();
        assert_eq!(ctx.base_pos, 0);
        assert_eq!(ctx.limits, Some(80..100));
        ctx.close_bounded_scope(scope);
        assert_eq!(ctx.base_pos, 0);
        assert_eq!(ctx.limits, Some(70..110));
    }

    #[test]
    fn bounded_scope_outside() {
        let mut ctx = Context::default();
        ctx.absolute_pos = 100;
        ctx.limits = Some(70..110);

        assert_eq!(ctx.bounded_scope(40), Err(ErrorKind::OutOfBounds.into()));
        assert_eq!(ctx.base_pos, 0);
        assert_eq!(ctx.absolute_pos, 100);
    }

    #[test]
    fn revision_scope_none() {
        let mut stream = GrowingMemoryStream::new();
        let mut ctx = Context::default();
        ctx.absolute_pos = 70;
        stream.seek(SeekFrom::Start(70)).unwrap();

        let scope = ctx.revision_scope(&mut stream, 30..40).unwrap();
        assert_eq!(ctx.base_pos, 30);
        assert_eq!(ctx.limits, Some(30..40));
        assert_eq!(stream.stream_position(), Ok(30));
        ctx.close_revision_scope(&mut stream, scope).unwrap();
        assert_eq!(ctx.base_pos, 0);
        assert_eq!(ctx.limits, None);
        assert_eq!(stream.stream_position(), Ok(70));
    }

    #[test]
    fn revision_scope_inside() {
        let mut stream = GrowingMemoryStream::new();
        let mut ctx = Context::default();
        ctx.absolute_pos = 80;
        ctx.limits = Some(70..110);
        stream.seek(SeekFrom::Start(80)).unwrap();

        let scope = ctx.revision_scope(&mut stream, 90..100).unwrap();
        assert_eq!(ctx.limits, Some(90..100));
        assert_eq!(stream.stream_position(), Ok(90));
        ctx.close_revision_scope(&mut stream, scope).unwrap();
        assert_eq!(ctx.limits, Some(70..110));
        assert_eq!(stream.stream_position(), Ok(80));
    }

    #[test]
    fn revision_scope_outside() {
        let mut stream = GrowingMemoryStream::new();
        let mut ctx = Context::default();
        ctx.absolute_pos = 80;
        ctx.limits = Some(70..110);
        stream.seek(SeekFrom::Start(80)).unwrap();

        assert_eq!(ctx.revision_scope(&mut stream, 90..120), Err(ErrorKind::OutOfBounds.into()));
        assert_eq!(stream.stream_position(), Ok(80));
        assert_eq!(ctx.absolute_pos, 80);
    }

    #[test]
    fn read_no_limit() {
        let mut stream = GrowingMemoryStream::new();
        stream.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut ctx = Context::default();
        ctx.absolute_pos = 3;
        stream.seek(SeekFrom::Start(3)).unwrap();

        let mut buffer = [0u8; 3];
        ctx.read(&mut stream, &mut buffer).unwrap();
        assert_eq!(ctx.absolute_pos, 6);
        assert_eq!(buffer, [3, 4, 5]);
        assert_eq!(stream.stream_position(), Ok(6));
    }

    #[test]
    fn read_inside_limit() {
        let mut stream = GrowingMemoryStream::new();
        stream.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut ctx = Context::default();
        ctx.limits = Some(2..7);
        ctx.absolute_pos = 3;
        stream.seek(SeekFrom::Start(3)).unwrap();

        let mut buffer = [0u8; 3];
        ctx.read(&mut stream, &mut buffer).unwrap();
        assert_eq!(ctx.absolute_pos, 6);
        assert_eq!(buffer, [3, 4, 5]);
        assert_eq!(stream.stream_position(), Ok(6));
    }

    #[test]
    fn read_outside_limit() {
        let mut stream = GrowingMemoryStream::new();
        stream.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut ctx = Context::default();
        ctx.limits = Some(2..5);
        ctx.absolute_pos = 3;
        stream.seek(SeekFrom::Start(3)).unwrap();

        let mut buffer = [0u8; 3];
        assert_eq!(ctx.read(&mut stream, &mut buffer), Err(ErrorKind::OutOfBounds.into()));
        assert_eq!(ctx.absolute_pos, 3);
        assert_eq!(buffer, [0, 0, 0]);
        assert_eq!(stream.stream_position(), Ok(3));
    }

    #[test]
    fn write_no_limit() {
        let mut stream = GrowingMemoryStream::new();
        stream.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut ctx = Context::default();
        ctx.absolute_pos = 3;
        stream.seek(SeekFrom::Start(3)).unwrap();

        let mut buffer = [0u8; 3];
        ctx.write(&mut stream, &mut buffer).unwrap();
        assert_eq!(ctx.absolute_pos, 6);
        assert_eq!(stream.stream_position(), Ok(6));
        assert_eq!(&stream.take(), &[0, 1, 2, 0, 0, 0, 6, 7]);
    }

    #[test]
    fn write_inside_limit() {
        let mut stream = GrowingMemoryStream::new();
        stream.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut ctx = Context::default();
        ctx.limits = Some(2..7);
        ctx.absolute_pos = 3;
        stream.seek(SeekFrom::Start(3)).unwrap();

        let mut buffer = [0u8; 3];
        ctx.write(&mut stream, &mut buffer).unwrap();
        assert_eq!(ctx.absolute_pos, 6);
        assert_eq!(stream.stream_position(), Ok(6));
        assert_eq!(&stream.take(), &[0, 1, 2, 0, 0, 0, 6, 7]);
    }

    #[test]
    fn write_outside_limit() {
        let mut stream = GrowingMemoryStream::new();
        stream.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
        let mut ctx = Context::default();
        ctx.limits = Some(2..5);
        ctx.absolute_pos = 3;
        stream.seek(SeekFrom::Start(3)).unwrap();

        let mut buffer = [0u8; 3];
        assert_eq!(ctx.write(&mut stream, &mut buffer), Err(ErrorKind::OutOfBounds.into()));
        assert_eq!(ctx.absolute_pos, 3);
        assert_eq!(buffer, [0, 0, 0]);
        assert_eq!(stream.stream_position(), Ok(3));
    }
}
