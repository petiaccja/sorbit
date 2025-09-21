use crate::io::{PartialStream, Read, Seek, SeekFrom, Write};
use crate::serialize::DeferredSerializer;

use super::Serializer;
use crate::byte_order::ByteOrder;
use crate::error::Error;

pub struct StreamSerializer<Stream: Write> {
    stream: Option<Stream>,
    // New items will be serialized using this byte order.
    byte_order: ByteOrder,
    // The current length of the stream.
    stream_len: u64,
    // The offset into `buffer` at which the current composite object begins.
    // This is important for alignment and padding within the composite.
    composite_base: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section(core::ops::Range<u64>);

macro_rules! to_xe_bytes {
    ($value:expr, $byte_order:expr) => {
        match $byte_order {
            ByteOrder::BigEndian => $value.to_be_bytes(),
            ByteOrder::LittleEndian => $value.to_le_bytes(),
        }
    };
}

const UNWRAP_STREAM_MSG: &'static str = "self must always have a stream, except when borrowed by a nesting";

impl<Stream: Write> StreamSerializer<Stream> {
    /// Create a new serializer.
    ///
    /// The default byte order is **big endian**. Use the [`Self::big_endian`] and
    /// [`Self::little_endian`] functions to set a specific byte order:
    /// ```
    /// # use serde_binfmt::serialize::StreamSerializer;
    /// # use serde_binfmt::io::GrowingMemoryStream;
    /// # let stream = GrowingMemoryStream::new();
    /// let serializer = StreamSerializer::new(stream).little_endian();
    /// ```
    pub fn new(stream: Stream) -> Self {
        Self { stream: Some(stream), byte_order: ByteOrder::BigEndian, stream_len: 0, composite_base: 0 }
    }

    /// Create a new serializer that uses the **big endian** byte order.
    pub fn big_endian(self) -> Self {
        Self { byte_order: ByteOrder::BigEndian, ..self }
    }

    /// Create a new serializer that uses the **little endian** byte order.
    pub fn little_endian(self) -> Self {
        Self { byte_order: ByteOrder::LittleEndian, ..self }
    }

    /// Take the serialized bytes from the serializer.
    pub fn take(self) -> Stream {
        self.stream.expect(UNWRAP_STREAM_MSG)
    }

    fn nest<O>(
        &mut self,
        serialize_members: impl FnOnce(&mut <Self as Serializer>::Nested) -> Result<O, <Self as Serializer>::Error>,
        change_byte_order: Option<ByteOrder>,
        change_base: Option<u64>,
    ) -> Result<<Self as Serializer>::Ok, <Self as Serializer>::Error> {
        // Borrow self's buffer and create a nested serializer.
        let mut nested = Self {
            stream: self.stream.take(),
            byte_order: change_byte_order.unwrap_or(self.byte_order),
            stream_len: self.stream_len,
            composite_base: change_base.unwrap_or(self.composite_base),
        };
        let start_pos = self.stream_len;
        let result = serialize_members(&mut nested);
        // Explode nested and restore self's buffer.
        // Nested's byte order and base are discarded.
        {
            let Self { stream, byte_order: _, stream_len, composite_base: _ } = nested;
            self.stream = stream;
            self.stream_len = stream_len;
        };
        result.map(|_| Section(start_pos..self.stream_len))
    }

    fn write(&mut self, bytes: &[u8]) -> Result<<Self as Serializer>::Ok, <Self as Serializer>::Error> {
        let start_pos = self.stream_len;
        let stream = self.stream.as_mut().expect(UNWRAP_STREAM_MSG);
        let result = stream.write(bytes);
        if result.is_ok() {
            self.stream_len += bytes.len() as u64;
        }
        result.map(|_| Section(start_pos..self.stream_len))
    }

    fn write_until(&mut self, until: u64, value: u8) -> Result<<Self as Serializer>::Ok, <Self as Serializer>::Error> {
        let start_pos = self.stream_len;
        let mut num_to_write = until as i64 - self.stream_len as i64;
        if num_to_write > 0 {
            while num_to_write >= 64 as i64 {
                self.write(&[value; 64])?;
                num_to_write -= 64;
            }
            while num_to_write > 0 as i64 {
                self.write(&[value])?;
                num_to_write -= 1;
            }
            Ok(Section(start_pos..self.stream_len))
        } else {
            Err(Error::LengthExceedsPadding)
        }
    }

    fn get_composite_len(&self) -> u64 {
        self.stream_len - self.composite_base
    }
}

impl<Stream: Write> Serializer for StreamSerializer<Stream> {
    type Ok = Section;
    type Error = Error;
    type Nested = Self;

    fn serialize_bool(&mut self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.write(&[value as u8])
    }

    fn serialize_u8(&mut self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_u16(&mut self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_u32(&mut self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_u64(&mut self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_i8(&mut self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_i16(&mut self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_i32(&mut self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_i64(&mut self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.write(&to_xe_bytes!(value, self.byte_order))
    }

    fn serialize_array<const N: usize>(&mut self, value: &[u8; N]) -> Result<Self::Ok, Self::Error> {
        self.write(value)
    }

    fn serialize_slice(&mut self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write(value)
    }

    fn serialize_composite<O>(
        &mut self,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<O, Self::Error>,
    ) -> Result<Self::Ok, Self::Error> {
        self.nest(serialize_members, None, Some(self.stream_len))
    }

    fn change_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        serialize_members: impl FnOnce(&mut Self::Nested) -> Result<O, Self::Error>,
    ) -> Result<Self::Ok, Self::Error> {
        self.nest(serialize_members, Some(byte_order), None)
    }

    fn pad(&mut self, until: u64) -> Result<Self::Ok, Self::Error> {
        let global_until = self.composite_base + until;
        self.write_until(global_until, 0)
    }

    fn align(&mut self, multiple_of: u64) -> Result<Self::Ok, Self::Error> {
        let len = self.get_composite_len();
        let aligned_len = (len + multiple_of - 1) / multiple_of * multiple_of;
        let global_until = self.composite_base + aligned_len;
        self.write_until(global_until, 0)
    }
}

impl<Stream: Read + Write + Seek> DeferredSerializer for StreamSerializer<Stream> {
    type SectionSerializer = StreamSerializer<PartialStream<Stream>>;
    type SectionReader = PartialStream<Stream>;

    fn update_section<O>(
        &mut self,
        section: &Self::Ok,
        update_section: impl FnOnce(&mut Self::SectionSerializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        let range = &section.0;
        let stream_pos = self.stream.as_mut().expect(UNWRAP_STREAM_MSG).stream_position()?;
        let stream = self.stream.take().expect(UNWRAP_STREAM_MSG);
        let partial_stream = match PartialStream::new(stream, range.clone()) {
            Ok(partial_stream) => partial_stream,
            Err(stream) => {
                self.stream.replace(stream);
                return Err(Error::EndOfFile);
            }
        };
        let mut section_serializer = StreamSerializer::new(partial_stream);
        let result = update_section(&mut section_serializer);
        {
            let Self::SectionSerializer { stream, byte_order: _, stream_len: _, composite_base: _ } =
                section_serializer;
            self.stream = stream.expect(UNWRAP_STREAM_MSG).into_inner().into();
            self.stream.as_mut().expect(UNWRAP_STREAM_MSG).seek(SeekFrom::Start(stream_pos))?;
        };
        result
    }

    fn read_section<Output>(
        &mut self,
        section: &Self::Ok,
        analyze_bytes: impl FnOnce(&mut Self::SectionReader) -> Output,
    ) -> Result<Output, Self::Error> {
        let range = &section.0;
        let stream_pos = self.stream.as_mut().expect(UNWRAP_STREAM_MSG).stream_position()?;
        let stream = self.stream.take().expect(UNWRAP_STREAM_MSG);
        let mut partial_stream = match PartialStream::new(stream, range.clone()) {
            Ok(partial_stream) => partial_stream,
            Err(stream) => {
                self.stream.replace(stream);
                return Err(Error::EndOfFile);
            }
        };
        let result = analyze_bytes(&mut partial_stream);
        self.stream = partial_stream.into_inner().into();
        self.stream.as_mut().expect(UNWRAP_STREAM_MSG).seek(SeekFrom::Start(stream_pos))?;
        Ok(result)
    }
}

impl crate::serialize::Section for Section {
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

impl From<Section> for () {
    fn from(_value: Section) -> Self {
        ()
    }
}

#[cfg(test)]
mod tests {
    use crate::io::GrowingMemoryStream;

    use super::*;

    //--------------------------------------------------------------------------
    // bool
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_bool() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
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
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u8(0xDE)?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u16_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u16(0xDEAD)?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD]);
        Ok(())
    }

    #[test]
    fn serialize_u32_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u32(0xDEADBEEF)?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_u64_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u64(0xDEADBEEF_FEEDDEAF)?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // i* be
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_i8_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_i8(0xDE_u8.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i16_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_i16(0xDEAD_u16.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD]);
        Ok(())
    }

    #[test]
    fn serialize_i32_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_i32(0xDEADBEEF_u32.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_i64_be() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_i64(0xDEADBEEF_FEEDDEAF_u64.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // u* le
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_u8_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_u8(0xDE)?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u16_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_u16(0xDEAD)?;
        assert_eq!(s.take().take(), vec![0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_u32_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_u32(0xDEADBEEF)?;
        assert_eq!(s.take().take(), vec![0xEF, 0xBE, 0xAD, 0xDE,]);
        Ok(())
    }

    #[test]
    fn serialize_u64_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_u64(0xDEADBEEF_FEEDDEAF)?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // i* le
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_i8_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_i8(0xDE_u8.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i16_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_i16(0xDEAD_u16.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i32_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_i32(0xDEADBEEF_u32.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xEF, 0xBE, 0xAD, 0xDE]);
        Ok(())
    }

    #[test]
    fn serialize_i64_le() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_i64(0xDEADBEEF_FEEDDEAF_u64.cast_signed())?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Array & slice
    //--------------------------------------------------------------------------

    #[test]
    fn serialize_array() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_array(&[0xAF, 0xDE, 0xED])?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED]);
        Ok(())
    }
    #[test]
    fn serialize_slice() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).little_endian();
        s.serialize_slice(&[0xAF, 0xDE, 0xED])?;
        assert_eq!(s.take().take(), vec![0xAF, 0xDE, 0xED]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Composites
    //--------------------------------------------------------------------------
    #[test]
    fn serialize_composite() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u8(0xEE)?;
        s.serialize_composite(|s| s.serialize_u16(0xAABB))?;
        s.serialize_u8(0xFF)?;
        assert_eq!(s.take().take(), vec![0xEE, 0xAA, 0xBB, 0xFF]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Byte order
    //--------------------------------------------------------------------------
    #[test]
    fn change_byte_order() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u16(0xEEFF)?;
        s.change_byte_order(ByteOrder::LittleEndian, |s| s.serialize_u16(0xAABB))?;
        s.serialize_u16(0xFFEE)?;
        assert_eq!(s.take().take(), vec![0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE]);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Padding
    //--------------------------------------------------------------------------
    #[test]
    fn pad_top_level() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_u8(0xEE)?;
        s.pad(4)?;
        assert_eq!(s.take().take(), vec![0xEE, 0x00, 0x00, 0x00]);
        Ok(())
    }

    #[test]
    fn pad_length_exceeds_padding() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_array(&[0xAA, 0xBB, 0xCC])?;
        assert_eq!(s.pad(2), Err(Error::LengthExceedsPadding));
        Ok(())
    }

    #[test]
    fn pad_composite() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
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
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_array(&[0x62, 0x85, 0x28, 0x75, 0x27])?;
        s.align(4)?;
        s.serialize_bool(true)?;
        assert_eq!(s.take().take(), vec![0x62, 0x85, 0x28, 0x75, 0x27, 0x00, 0x00, 0x00, 0x01]);
        Ok(())
    }

    #[test]
    fn align_composite() -> Result<(), Error> {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new()).big_endian();
        s.serialize_bool(true)?;
        s.serialize_composite(|s| {
            s.serialize_array(&[0x62, 0x85, 0x28, 0x75, 0x27])?;
            s.align(4)
        })?;
        s.serialize_bool(true)?;
        assert_eq!(s.take().take(), vec![0x01, 0x62, 0x85, 0x28, 0x75, 0x27, 0x00, 0x00, 0x00, 0x01]);
        Ok(())
    }
}
