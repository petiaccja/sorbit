use std::convert::Infallible;

use crate::io::{Read, Seek, SeekFrom, StreamSection, Write};
use crate::ser_de::RevisableSerializer;

use crate::byte_order::ByteOrder;
use crate::error::{Error, ErrorKind};
use crate::ser_de::Serializer;
use crate::stream_ser_de::context::Context;

/// A [`Serializer`] that works with any [`Write`]-able stream.
///
/// The stream can be anything, a file, a TCP stream, or an in-memory
/// buffer.
///
/// For streams that also implement both [`Read`] and [`Seek`], the serializer
/// is also a [`RevisableSerializer`](sorbit::ser_de::RevisableSerializer).
pub struct StreamSerializer<Stream: Write> {
    stream: Stream,
    // The current length of the stream.
    context: Context,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeSpan(core::ops::Range<u64>);

macro_rules! to_xe_bytes {
    ($value:expr, $byte_order:expr) => {
        match $byte_order {
            ByteOrder::BigEndian => $value.to_be_bytes(),
            ByteOrder::LittleEndian => $value.to_le_bytes(),
        }
    };
}

impl<Stream: Write> StreamSerializer<Stream> {
    /// Create a new serializer.
    ///
    /// The default byte order is native byte order. Use the
    /// [`change_byte_order`](Self::change_byte_order) to set a specific byte order:
    /// ```
    /// # use sorbit::stream_ser_de::StreamSerializer;
    /// # use sorbit::io::GrowingMemoryStream;
    /// # use sorbit::byte_order::ByteOrder;
    /// # let stream = GrowingMemoryStream::new();
    /// let serializer = StreamSerializer::new(stream).change_byte_order(ByteOrder::LittleEndian);
    /// ```
    pub fn new(stream: Stream) -> Self {
        Self { stream, context: Context::default() }
    }

    /// Create a new serializer that uses the specified byte order.
    pub fn change_byte_order(self, byte_order: ByteOrder) -> Self {
        let context = self.context.change_byte_order(byte_order);
        Self { context, ..self }
    }

    /// Take the serialized bytes from the serializer.
    pub fn take(self) -> Stream {
        self.stream
    }

    fn write(&mut self, bytes: &[u8]) -> Result<RangeSpan, Error> {
        self.context.write(&mut self.stream, bytes).map(|range| RangeSpan(range))
    }

    fn write_until(&mut self, until: u64, value: u8) -> Result<RangeSpan, Error> {
        let padding: [u8; 64] = [value; 64];
        if until < self.context.local_pos() {
            return Err(ErrorKind::LengthExceedsPadding.into());
        }
        let start = self.context.absolute_pos();
        while self.context.local_pos() < until {
            let count = core::cmp::min(padding.len() as u64, until - self.context.local_pos()) as usize;
            self.write(&padding[0..count])?;
        }
        let end = self.context.absolute_pos();
        let span = RangeSpan(start..end);
        Ok(span)
    }
}

impl<Stream: Write> Serializer for StreamSerializer<Stream> {
    type Success = RangeSpan;
    type Error = Error;

    fn success(&mut self) -> Result<Self::Success, Self::Error> {
        self.write(&[])
    }

    fn error(&mut self, message: &'static str) -> Result<Infallible, Self::Error> {
        Err(ErrorKind::Custom(message).into())
    }

    fn serialize_bool(&mut self, value: bool) -> Result<Self::Success, Self::Error> {
        self.write(&[value as u8])
    }

    fn serialize_u8(&mut self, value: u8) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_u16(&mut self, value: u16) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_u32(&mut self, value: u32) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_u64(&mut self, value: u64) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_u128(&mut self, value: u128) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_i8(&mut self, value: i8) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_i16(&mut self, value: i16) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_i32(&mut self, value: i32) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_i64(&mut self, value: i64) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_i128(&mut self, value: i128) -> Result<Self::Success, Self::Error> {
        self.write(&to_xe_bytes!(value, self.context.byte_order()))
    }

    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<Self::Success, Self::Error> {
        self.write(value)
    }

    fn serialize_slice(&mut self, value: &[u8]) -> Result<Self::Success, Self::Error> {
        self.write(value)
    }

    fn pad(&mut self, until: u64) -> Result<Self::Success, Self::Error> {
        self.write_until(until, 0)
    }

    fn align(&mut self, multiple_of: u64) -> Result<Self::Success, Self::Error> {
        let until = (self.context.local_pos() + multiple_of - 1) / multiple_of * multiple_of;
        self.pad(until)
    }

    fn serialize_composite<Output>(
        &mut self,
        serialize_members: impl FnOnce(&mut Self) -> Result<Output, Self::Error>,
    ) -> Result<(Self::Success, Output), Self::Error> {
        let scope = self.context.composite_scope();
        let start = self.context.absolute_pos();
        let result = serialize_members(self);
        let end = self.context.absolute_pos();
        self.context.close_composite_scope(scope);
        let span = RangeSpan(start..end);
        result.map(|output| (span, output))
    }

    fn with_byte_order<Output>(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self) -> Result<Output, Self::Error>,
    ) -> Result<Output, Self::Error> {
        let scope = self.context.byte_order_scope(byte_order);
        let result = serialize_members(self);
        self.context.close_byte_order_scope(scope);
        result
    }
}

impl<Stream> RevisableSerializer for StreamSerializer<Stream>
where
    Stream: Read + Write + Seek,
{
    fn revise_span<Output>(
        &mut self,
        span: &Self::Success,
        update_section: impl FnOnce(&mut Self) -> Result<Output, Self::Error>,
    ) -> Result<Output, Self::Error> {
        let scope = self.context.revision_scope(&mut self.stream, span.0.clone())?;
        let result = update_section(self);
        self.context.close_revision_scope(&mut self.stream, scope)?;
        result
    }

    fn analyze_span<Output, Error, AnalyzeSpanFn>(
        &mut self,
        section: &Self::Success,
        analyze_span_fn: AnalyzeSpanFn,
    ) -> Result<Output, Self::Error>
    where
        AnalyzeSpanFn: for<'analyze> FnOnce(&mut dyn Read) -> Result<Output, Error>,
        Error: Into<Self::Error>,
    {
        let range = &section.0;
        let stream_pos = self.stream.stream_position()?;
        let mut partial_stream =
            StreamSection::new(&mut self.stream, range.clone()).map_err(|_| ErrorKind::UnexpectedEof)?;
        let result = analyze_span_fn(&mut partial_stream);
        self.stream.seek(SeekFrom::Start(stream_pos))?;
        result.map_err(|err| err.into())
    }
}

impl crate::ser_de::Span for RangeSpan {
    fn start(&self) -> u64 {
        self.0.start
    }
    fn end(&self) -> u64 {
        self.0.end
    }
    fn len(&self) -> u64 {
        self.end() - self.start()
    }
}

impl From<RangeSpan> for () {
    fn from(_value: RangeSpan) -> Self {
        ()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{error::ErrorKind, io::GrowingMemoryStream};

    use super::*;

    //--------------------------------------------------------------------------
    // bool
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_bool() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_bool(true)?;
        s.serialize_bool(false)?;
        assert_eq!(s.take().take(), vec![1, 0]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // u* be
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_u8_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u8(0xDE)?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u16_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u16(0xDEAD)?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD]);
        Ok(())
    }

    #[test]
    fn serialize_u32_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u32(0xDEADBEEF)?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_u64_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u64(0xDEADBEEF_FEEDDEAF)?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]);
        Ok(())
    }

    #[test]
    fn serialize_u128_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u128(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAF)?;
        assert_eq!(
            s.take().take(),
            vec![
                0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF, 0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF
            ]
        );
        Ok(())
    }

    //--------------------------------------------------------------------------
    // i* be
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_i8_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_i8(0xDE_u8.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i16_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_i16(0xDEAD_u16.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD]);
        Ok(())
    }

    #[test]
    fn serialize_i32_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_i32(0xDEADBEEF_u32.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_i64_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_i64(0xDEADBEEF_FEEDDEAF_u64.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]);
        Ok(())
    }

    #[test]
    fn serialize_i128_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_i128(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAFu128.cast_signed())?;
        assert_eq!(
            s.take().take(),
            vec![
                0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF, 0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF
            ]
        );
        Ok(())
    }

    //--------------------------------------------------------------------------
    // u* le
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_u8_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_u8(0xDE)?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u16_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_u16(0xDEAD)?;
        assert_eq!(s.take().take(), vec![0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u32_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_u32(0xDEADBEEF)?;
        assert_eq!(s.take().take(), vec![0xEF, 0xBE, 0xAD, 0xDE,]);
        Ok(())
    }

    #[test]
    fn serialize_u64_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_u64(0xDEADBEEF_FEEDDEAF)?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u128_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_u128(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAF)?;
        assert_eq!(
            s.take().take(),
            vec![
                0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE, 0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE
            ]
        );
        Ok(())
    }

    //--------------------------------------------------------------------------
    // i* le
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_i8_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_i8(0xDE_u8.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i16_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_i16(0xDEAD_u16.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i32_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_i32(0xDEADBEEF_u32.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xEF, 0xBE, 0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i64_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_i64(0xDEADBEEF_FEEDDEAF_u64.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i128_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_i128(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAFu128.cast_signed())?;
        assert_eq!(
            s.take().take(),
            vec![
                0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE, 0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE
            ]
        );
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Array & slice
    //--------------------------------------------------------------------------

    #[test]
    fn serialize_array() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_array(&[0xAF, 0xDE, 0xED])?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED]);
        Ok(())
    }
    #[test]
    fn serialize_slice() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::LittleEndian);
        s.serialize_slice(&[0xAF, 0xDE, 0xED])?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Padding
    //--------------------------------------------------------------------------

    #[test]
    fn pad_top_level() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u8(0xEE)?;
        s.pad(4)?;
        assert_eq!(s.take().take(), vec![0xEE, 0x00, 0x00, 0x00]);
        Ok(())
    }

    #[test]
    fn pad_length_exceeds_padding() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_array(&[0xAA, 0xBB, 0xCC])?;
        assert_eq!(s.pad(2), Err(ErrorKind::LengthExceedsPadding.into()));
        Ok(())
    }

    #[test]
    fn pad_composite() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_array(&[0xAA, 0xBB, 0xCC])?;
        s.serialize_composite(|s| {
            s.serialize_bool(true)?;
            s.pad(4)
        })?;
        s.serialize_u8(0xAF)?;
        assert_eq!(s.take().take(), vec![0xAA, 0xBB, 0xCC, 0x01, 0x00, 0x00, 0x00, 0xAF]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Alignment
    //--------------------------------------------------------------------------
    #[test]
    fn align_top_level() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_array(&[0x62, 0x85, 0x28, 0x75, 0x27])?;
        s.align(4)?;
        s.serialize_bool(true)?;
        assert_eq!(s.take().take(), vec![0x62, 0x85, 0x28, 0x75, 0x27, 0x00, 0x00, 0x00, 0x01]);
        Ok(())
    }

    #[test]
    fn align_composite() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_bool(true)?;
        s.serialize_composite(|s| {
            s.serialize_array(&[0x62, 0x85, 0x28, 0x75, 0x27])?;
            s.align(4)
        })?;
        s.serialize_bool(true)?;
        assert_eq!(s.take().take(), vec![0x01, 0x62, 0x85, 0x28, 0x75, 0x27, 0x00, 0x00, 0x00, 0x01]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Composites
    //--------------------------------------------------------------------------

    #[rstest]
    #[case(ByteOrder::LittleEndian, vec![0xEE, 0xBB, 0xAA, 0xFF])]
    #[case(ByteOrder::BigEndian, vec![0xEE, 0xAA, 0xBB, 0xFF])]
    fn serialize_composite(#[case] byte_order: ByteOrder, #[case] expected: Vec<u8>) -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(byte_order);
        s.serialize_u8(0xEE)?;
        s.serialize_composite(|s| s.serialize_u16(0xAABB))?;
        s.serialize_u8(0xFF)?;
        assert_eq!(s.take().take(), expected);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Byte order
    //--------------------------------------------------------------------------

    #[rstest]
    #[case(ByteOrder::LittleEndian, vec![0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE])]
    #[case(ByteOrder::BigEndian, vec![0xEE, 0xFF, 0xAA, 0xBB, 0xFF, 0xEE])]
    fn change_byte_order(#[case] byte_order: ByteOrder, #[case] expected: Vec<u8>) -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(ByteOrder::BigEndian);
        s.serialize_u16(0xEEFF)?;
        s.with_byte_order(byte_order, |s| s.serialize_u16(0xAABB))?;
        s.serialize_u16(0xFFEE)?;
        assert_eq!(s.take().take(), expected);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Revise span
    //--------------------------------------------------------------------------

    #[rstest]
    #[case(ByteOrder::LittleEndian, vec![0xBB, 0xAA])]
    #[case(ByteOrder::BigEndian, vec![ 0xAA, 0xBB])]
    fn revise_span(#[case] byte_order: ByteOrder, #[case] expected: Vec<u8>) -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).change_byte_order(byte_order);
        let span = s.serialize_u16(0x0000)?;
        s.revise_span(&span, |s| s.serialize_u16(0xAABB))?;
        assert_eq!(s.take().take(), expected);
        Ok(())
    }
}
