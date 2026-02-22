use crate::{
    byte_order::ByteOrder,
    deserialize::Deserializer,
    error::{Error, ErrorKind},
    io::Read,
};

pub struct StreamDeserializer<Stream: Read> {
    stream: Option<Stream>,
    byte_order: ByteOrder,
    stream_pos: u64,     // The current position in the stream.
    composite_base: u64, // Marks the stream position at which the current composite begins.
}

macro_rules! from_xe_bytes {
    ($type:ty, $bytes:expr, $byte_order:expr) => {
        match $byte_order {
            ByteOrder::BigEndian => <$type>::from_be_bytes($bytes),
            ByteOrder::LittleEndian => <$type>::from_le_bytes($bytes),
        }
    };
}

const UNWRAP_STREAM_MSG: &'static str = "self must always have a stream, except when borrowed by a nesting";

impl<Stream: Read> StreamDeserializer<Stream> {
    /// Create a new deserializer.
    ///
    /// The default byte order is **big endian**. Use the [`Self::big_endian`] and
    /// [`Self::little_endian`] functions to set a specific byte order:
    /// ```
    /// # use sorbit::deserialize::StreamDeserializer;
    /// # use sorbit::io::GrowingMemoryStream;
    /// # let stream = GrowingMemoryStream::new();
    /// let serializer = StreamDeserializer::new(stream).little_endian();
    /// ```
    pub fn new(stream: Stream) -> Self {
        Self { stream: Some(stream), byte_order: ByteOrder::native(), stream_pos: 0, composite_base: 0 }
    }

    /// Create a new deserializer that uses the **big endian** byte order.
    pub fn big_endian(self) -> Self {
        Self { byte_order: ByteOrder::BigEndian, ..self }
    }

    /// Create a new deserializer that uses the **little endian** byte order.
    pub fn little_endian(self) -> Self {
        Self { byte_order: ByteOrder::LittleEndian, ..self }
    }

    /// Create a new deserializer that uses the specified byte order.
    pub fn set_byte_order(self, byte_order: ByteOrder) -> Self {
        Self { byte_order, ..self }
    }

    /// Take the serialized bytes from the serializer.
    pub fn take(self) -> Stream {
        self.stream.expect(UNWRAP_STREAM_MSG)
    }

    fn read_fixed<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut bytes = [0u8; N];
        self.read(&mut bytes).map(|_| bytes)
    }

    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        self.stream.as_mut().expect(UNWRAP_STREAM_MSG).read(bytes)?;
        self.stream_pos += bytes.len() as u64;
        Ok(())
    }

    fn read_until(&mut self, until: u64) -> Result<(), Error> {
        let mut num_to_ignore = until as i64 - self.stream_pos as i64;
        if num_to_ignore >= 0 {
            while num_to_ignore >= 64 as i64 {
                self.read(&mut [0; 64])?;
                num_to_ignore -= 64;
            }
            while num_to_ignore > 0 as i64 {
                self.read(&mut [0])?;
                num_to_ignore -= 1;
            }
            Ok(())
        } else {
            Err(ErrorKind::LengthExceedsPadding.into())
        }
    }

    fn current_composite_len(&self) -> u64 {
        self.stream_pos - self.composite_base
    }

    fn nest<O>(
        &mut self,
        deserialize_members: impl FnOnce(&mut Self) -> Result<O, Error>,
        change_byte_order: Option<ByteOrder>,
        change_base: Option<u64>,
    ) -> Result<O, Error> {
        // Borrow self's buffer and create a nested serializer.
        let mut nested = Self {
            stream: self.stream.take(),
            byte_order: change_byte_order.unwrap_or(self.byte_order),
            stream_pos: self.stream_pos,
            composite_base: change_base.unwrap_or(self.composite_base),
        };
        let result = deserialize_members(&mut nested);
        // Explode nested and restore self's buffer.
        // Nested's byte order and base are discarded.
        {
            let Self { stream, byte_order: _, stream_pos: stream_len, composite_base: _ } = nested;
            self.stream = stream;
            self.stream_pos = stream_len;
        };
        result
    }
}

impl<Stream: Read> Deserializer for StreamDeserializer<Stream> {
    type Error = Error;
    type CompositeDeserializer = Self;
    type ByteOrderDeserializer = Self;

    fn deserialize_bool(&mut self) -> Result<bool, Self::Error> {
        let byte: [u8; 1] = self.read_fixed()?;
        match byte[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ErrorKind::InvalidEnumVariant.into()),
        }
    }

    fn deserialize_u8(&mut self) -> Result<u8, Self::Error> {
        Ok(from_xe_bytes!(u8, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_u16(&mut self) -> Result<u16, Self::Error> {
        Ok(from_xe_bytes!(u16, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(from_xe_bytes!(u32, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(from_xe_bytes!(u64, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_i8(&mut self) -> Result<i8, Self::Error> {
        Ok(from_xe_bytes!(i8, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_i16(&mut self) -> Result<i16, Self::Error> {
        Ok(from_xe_bytes!(i16, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_i32(&mut self) -> Result<i32, Self::Error> {
        Ok(from_xe_bytes!(i32, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_i64(&mut self) -> Result<i64, Self::Error> {
        Ok(from_xe_bytes!(i64, self.read_fixed()?, self.byte_order))
    }

    fn deserialize_array<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        self.read_fixed()
    }

    fn deserialize_slice(&mut self, value: &mut [u8]) -> Result<(), Self::Error> {
        self.read(value)
    }

    fn pad(&mut self, until: u64) -> Result<(), Self::Error> {
        let absolute_until = self.composite_base + until;
        self.read_until(absolute_until)
    }

    fn align(&mut self, multiple_of: u64) -> Result<(), Self::Error> {
        let len = self.current_composite_len();
        let aligned_len = (len + multiple_of - 1) / multiple_of * multiple_of;
        let absolute_until = self.composite_base + aligned_len;
        self.read_until(absolute_until)
    }

    fn deserialize_composite<O>(
        &mut self,
        deserialize_members: impl FnOnce(&mut Self::CompositeDeserializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        self.nest(deserialize_members, None, Some(self.stream_pos))
    }

    fn with_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        deserialize_members: impl FnOnce(&mut Self::ByteOrderDeserializer) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        self.nest(deserialize_members, Some(byte_order), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        error::ErrorKind,
        io::{FixedMemoryStream, Seek},
    };

    //--------------------------------------------------------------------------
    // bool
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_bool() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0u8, 1u8, 45u8]));
        assert_eq!(s.deserialize_bool(), Ok(false));
        assert_eq!(s.deserialize_bool(), Ok(true));
        assert_eq!(s.deserialize_bool(), Err(ErrorKind::InvalidEnumVariant.into()));
    }

    //--------------------------------------------------------------------------
    // u* be
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_u8_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).big_endian();
        assert_eq!(s.deserialize_u8(), Ok(0xDE));
    }

    #[test]
    fn deserialize_u16_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD])).big_endian();
        assert_eq!(s.deserialize_u16(), Ok(0xDEAD));
    }

    #[test]
    fn deserialize_u32_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF])).big_endian();
        assert_eq!(s.deserialize_u32(), Ok(0xDEADBEEF));
    }

    #[test]
    fn deserialize_u64_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]))
            .big_endian();
        assert_eq!(s.deserialize_u64(), Ok(0xDEADBEEF_FEEDDEAF));
    }

    //--------------------------------------------------------------------------
    // i* be
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_i8_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).big_endian();
        assert_eq!(s.deserialize_i8(), Ok(0xDE_u8.cast_signed()));
    }

    #[test]
    fn deserialize_i16_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD])).big_endian();
        assert_eq!(s.deserialize_i16(), Ok(0xDEAD_u16.cast_signed()));
    }

    #[test]
    fn deserialize_i32_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF])).big_endian();
        assert_eq!(s.deserialize_i32(), Ok(0xDEADBEEF_u32.cast_signed()));
    }

    #[test]
    fn deserialize_i64_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]))
            .big_endian();
        assert_eq!(s.deserialize_i64(), Ok(0xDEADBEEF_FEEDDEAF_u64.cast_signed()));
    }

    //--------------------------------------------------------------------------
    // u* le
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_u8_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).little_endian();
        assert_eq!(s.deserialize_u8(), Ok(0xDE));
    }

    #[test]
    fn deserialize_u16_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAD, 0xDE])).little_endian();
        assert_eq!(s.deserialize_u16(), Ok(0xDEAD));
    }

    #[test]
    fn deserialize_u32_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEF, 0xBE, 0xAD, 0xDE])).little_endian();
        assert_eq!(s.deserialize_u32(), Ok(0xDEADBEEF));
    }

    #[test]
    fn deserialize_u64_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]))
            .little_endian();
        assert_eq!(s.deserialize_u64(), Ok(0xDEADBEEF_FEEDDEAF));
    }

    //--------------------------------------------------------------------------
    // i* le
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_i8_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).little_endian();
        assert_eq!(s.deserialize_i8(), Ok(0xDE_u8.cast_signed()));
    }

    #[test]
    fn deserialize_i16_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAD, 0xDE])).little_endian();
        assert_eq!(s.deserialize_i16(), Ok(0xDEAD_u16.cast_signed()));
    }

    #[test]
    fn deserialize_i32_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEF, 0xBE, 0xAD, 0xDE])).little_endian();
        assert_eq!(s.deserialize_i32(), Ok(0xDEADBEEF_u32.cast_signed()));
    }

    #[test]
    fn deserialize_i64_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]))
            .little_endian();
        assert_eq!(s.deserialize_i64(), Ok(0xDEADBEEF_FEEDDEAF_u64.cast_signed()));
    }

    //--------------------------------------------------------------------------
    // Array & slice
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_array() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED])).little_endian();
        assert_eq!(s.deserialize_array(), Ok([0xAF, 0xDE, 0xED]));
    }

    #[test]
    fn deserialize_slice() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED])).little_endian();
        let mut slc = [0u8, 0u8, 0u8];
        assert_eq!(s.deserialize_slice(&mut slc), Ok(()));
        assert_eq!(slc, [0xAF, 0xDE, 0xED]);
    }

    //--------------------------------------------------------------------------
    // Composites
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_composite() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xAA, 0xBB, 0xFF])).big_endian();
        assert_eq!(s.deserialize_u8(), Ok(0xEE));
        assert_eq!(s.deserialize_composite(|s| { s.deserialize_u16() }), Ok(0xAABB));
        assert_eq!(s.deserialize_u8(), Ok(0xFF));
    }

    //--------------------------------------------------------------------------
    // Byte order
    //--------------------------------------------------------------------------
    #[test]
    fn change_byte_order() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE])).big_endian();
        assert_eq!(s.deserialize_u16(), Ok(0xEEFF));
        assert_eq!(s.with_byte_order(ByteOrder::LittleEndian, |s| s.deserialize_u16()), Ok(0xAABB));
        assert_eq!(s.deserialize_u16(), Ok(0xFFEE));
    }

    //--------------------------------------------------------------------------
    // Padding
    //--------------------------------------------------------------------------
    #[test]
    fn pad_top_level() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0x00, 0x00, 0x00])).big_endian();
        assert_eq!(s.deserialize_u8(), Ok(0xEE));
        assert_eq!(s.pad(4), Ok(()));
        assert_eq!(s.take().stream_position(), Ok(4));
    }

    #[test]
    fn pad_length_exceeds_padding() -> Result<(), Error> {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAA, 0xBB, 0xCC])).big_endian();
        s.deserialize_array::<3>()?;
        assert_eq!(s.pad(2), Err(ErrorKind::LengthExceedsPadding.into()));
        Ok(())
    }

    #[test]
    fn pad_composite() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAA, 0xBB, 0xCC, 0x01, 0x00, 0x00, 0x00, 0xAF]))
            .big_endian();
        assert_eq!(s.deserialize_array(), Ok([0xAA, 0xBB, 0xCC]));
        assert_eq!(
            s.deserialize_composite(|s| {
                let value = s.deserialize_bool()?;
                s.pad(4).map(|_| value)
            }),
            Ok(true)
        );
        assert_eq!(s.deserialize_u8(), Ok(0xAF));
    }

    //--------------------------------------------------------------------------
    // Alignment
    //--------------------------------------------------------------------------
    #[test]
    fn align_top_level() {
        let mut s =
            StreamDeserializer::new(FixedMemoryStream::new([0x62, 0x85, 0x28, 0x75, 0x27, 0x00, 0x00, 0x00, 0x01]));
        assert_eq!(s.deserialize_array(), Ok([0x62, 0x85, 0x28, 0x75, 0x27]));
        assert_eq!(s.align(4), Ok(()));
        assert_eq!(s.deserialize_bool(), Ok(true));
    }

    #[test]
    fn align_composite() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([
            0x01, 0x62, 0x85, 0x28, 0x75, 0x27, 0x00, 0x00, 0x00, 0x01,
        ]));
        assert_eq!(s.deserialize_bool(), Ok(true));
        assert_eq!(
            s.deserialize_composite(|s| {
                let value = s.deserialize_array()?;
                s.align(4).map(|_| value)
            }),
            Ok([0x62, 0x85, 0x28, 0x75, 0x27])
        );
        assert_eq!(s.deserialize_bool(), Ok(true));
    }
}
