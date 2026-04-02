use crate::{
    byte_order::ByteOrder,
    error::{Error, ErrorKind},
    io::Read,
    ser_de::Deserializer,
    stream_ser_de::context::Context,
};

/// A [`Deserializer`] that works with any [`Read`]-able stream.
///
/// The stream can be anything, a file, a TCP stream, or an in-memory
/// buffer.
pub struct StreamDeserializer<Stream: Read> {
    stream: Stream,
    context: Context,
}

macro_rules! from_xe_bytes {
    ($type:ty, $bytes:expr, $byte_order:expr) => {
        match $byte_order {
            ByteOrder::BigEndian => <$type>::from_be_bytes($bytes),
            ByteOrder::LittleEndian => <$type>::from_le_bytes($bytes),
        }
    };
}

impl<Stream: Read> StreamDeserializer<Stream> {
    /// Create a new deserializer.
    ///
    /// The default byte order is native byte order. Use the
    /// [`change_byte_order`](Self::change_byte_order) to set a specific byte order:
    /// ```
    /// # use sorbit::stream_ser_de::StreamDeserializer;
    /// # use sorbit::io::GrowingMemoryStream;
    /// # use sorbit::byte_order::ByteOrder;
    /// # let stream = GrowingMemoryStream::new();
    /// let serializer = StreamDeserializer::new(stream).change_byte_order(ByteOrder::LittleEndian);
    /// ```
    pub fn new(stream: Stream) -> Self {
        Self { stream, context: Context::default() }
    }

    /// Create a new deserializer that uses the specified byte order.
    pub fn change_byte_order(self, byte_order: ByteOrder) -> Self {
        let context = self.context.change_byte_order(byte_order);
        Self { context, ..self }
    }

    /// Take the serialized bytes from the serializer.
    pub fn take(self) -> Stream {
        self.stream
    }

    fn read_fixed<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut bytes = [0u8; N];
        self.read(&mut bytes).map(|_| bytes)
    }

    fn read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        self.context.read(&mut self.stream, bytes).map(|_| ())
    }

    fn read_until(&mut self, until: u64) -> Result<(), Error> {
        let mut padding: [u8; 64] = [0; 64];
        if until < self.context.local_pos() {
            return Err(ErrorKind::LengthExceedsPadding.into());
        }
        while self.context.local_pos() < until {
            let count = core::cmp::min(padding.len() as u64, until - self.context.local_pos()) as usize;
            self.read(&mut padding[0..count])?;
        }
        Ok(())
    }
}

impl<Stream: Read> Deserializer for StreamDeserializer<Stream> {
    type Error = Error;

    fn deserialize_bool(&mut self) -> Result<bool, Self::Error> {
        let byte: [u8; 1] = self.read_fixed()?;
        match byte[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ErrorKind::InvalidEnumVariant.into()),
        }
    }

    fn deserialize_u8(&mut self) -> Result<u8, Self::Error> {
        Ok(from_xe_bytes!(u8, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_u16(&mut self) -> Result<u16, Self::Error> {
        Ok(from_xe_bytes!(u16, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(from_xe_bytes!(u32, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(from_xe_bytes!(u64, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_u128(&mut self) -> Result<u128, Self::Error> {
        Ok(from_xe_bytes!(u128, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_i8(&mut self) -> Result<i8, Self::Error> {
        Ok(from_xe_bytes!(i8, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_i16(&mut self) -> Result<i16, Self::Error> {
        Ok(from_xe_bytes!(i16, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_i32(&mut self) -> Result<i32, Self::Error> {
        Ok(from_xe_bytes!(i32, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_i64(&mut self) -> Result<i64, Self::Error> {
        Ok(from_xe_bytes!(i64, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_i128(&mut self) -> Result<i128, Self::Error> {
        Ok(from_xe_bytes!(i128, self.read_fixed()?, self.context.byte_order()))
    }

    fn deserialize_array<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        self.read_fixed()
    }

    fn deserialize_slice(&mut self, value: &mut [u8]) -> Result<(), Self::Error> {
        self.read(value)
    }

    fn pad(&mut self, until: u64) -> Result<(), Self::Error> {
        self.read_until(until)
    }

    fn align(&mut self, multiple_of: u64) -> Result<(), Self::Error> {
        let until = (self.context.local_pos() + multiple_of - 1) / multiple_of * multiple_of;
        self.pad(until)
    }

    fn deserialize_composite<O>(
        &mut self,
        deserialize_members: impl FnOnce(&mut Self) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        let scope = self.context.composite_scope();
        let result = deserialize_members(self);
        self.context.close_composite_scope(scope);
        result
    }

    fn with_byte_order<O>(
        &mut self,
        byte_order: ByteOrder,
        deserialize_members: impl FnOnce(&mut Self) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        let scope = self.context.byte_order_scope(byte_order);
        let result = deserialize_members(self);
        self.context.close_byte_order_scope(scope);
        result
    }

    fn deserialize_bounded<O>(
        &mut self,
        byte_count: u64,
        deserialize_object: impl FnOnce(&mut Self) -> Result<O, Self::Error>,
    ) -> Result<O, Self::Error> {
        let scope = self.context.bounded_scope(byte_count)?;
        let result = deserialize_object(self);
        self.context.close_bounded_scope(scope);
        result
    }

    fn bytes_in_bounds(&self) -> Option<u64> {
        self.context.bytes_in_bounds()
    }

    fn error<O>(&self, message: &'static str) -> Result<O, Self::Error> {
        Err(Self::Error::from(ErrorKind::Custom(message)))
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
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u8(), Ok(0xDE));
    }

    #[test]
    fn deserialize_u16_be() {
        let mut s =
            StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD])).change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u16(), Ok(0xDEAD));
    }

    #[test]
    fn deserialize_u32_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u32(), Ok(0xDEADBEEF));
    }

    #[test]
    fn deserialize_u64_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u64(), Ok(0xDEADBEEF_FEEDDEAF));
    }

    #[test]
    fn deserialize_u128_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([
            0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF, 0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF,
        ]))
        .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u128(), Ok(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAF));
    }

    //--------------------------------------------------------------------------
    // i* be
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_i8_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_i8(), Ok(0xDE_u8.cast_signed()));
    }

    #[test]
    fn deserialize_i16_be() {
        let mut s =
            StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD])).change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_i16(), Ok(0xDEAD_u16.cast_signed()));
    }

    #[test]
    fn deserialize_i32_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_i32(), Ok(0xDEADBEEF_u32.cast_signed()));
    }

    #[test]
    fn deserialize_i64_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_i64(), Ok(0xDEADBEEF_FEEDDEAF_u64.cast_signed()));
    }

    #[test]
    fn deserialize_i128_be() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([
            0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF, 0xDE, 0xAD, 0xBE, 0xEF, 0xFE, 0xED, 0xDE, 0xAF,
        ]))
        .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_i128(), Ok(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAF_u128.cast_signed()));
    }

    //--------------------------------------------------------------------------
    // u* le
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_u8_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_u8(), Ok(0xDE));
    }

    #[test]
    fn deserialize_u16_le() {
        let mut s =
            StreamDeserializer::new(FixedMemoryStream::new([0xAD, 0xDE])).change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_u16(), Ok(0xDEAD));
    }

    #[test]
    fn deserialize_u32_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEF, 0xBE, 0xAD, 0xDE]))
            .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_u32(), Ok(0xDEADBEEF));
    }

    #[test]
    fn deserialize_u64_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]))
            .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_u64(), Ok(0xDEADBEEF_FEEDDEAF));
    }

    #[test]
    fn deserialize_u128_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([
            0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE, 0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE,
        ]))
        .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_u128(), Ok(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAF));
    }

    //--------------------------------------------------------------------------
    // i* le
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_i8_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xDE])).change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_i8(), Ok(0xDE_u8.cast_signed()));
    }

    #[test]
    fn deserialize_i16_le() {
        let mut s =
            StreamDeserializer::new(FixedMemoryStream::new([0xAD, 0xDE])).change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_i16(), Ok(0xDEAD_u16.cast_signed()));
    }

    #[test]
    fn deserialize_i32_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEF, 0xBE, 0xAD, 0xDE]))
            .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_i32(), Ok(0xDEADBEEF_u32.cast_signed()));
    }

    #[test]
    fn deserialize_i64_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE]))
            .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_i64(), Ok(0xDEADBEEF_FEEDDEAF_u64.cast_signed()));
    }

    #[test]
    fn deserialize_i128_le() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([
            0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE, 0xAF, 0xDE, 0xED, 0xFE, 0xEF, 0xBE, 0xAD, 0xDE,
        ]))
        .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_i128(), Ok(0xDEADBEEF_FEEDDEAF_DEADBEEF_FEEDDEAFu128.cast_signed()));
    }

    //--------------------------------------------------------------------------
    // Array & slice
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_array() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED]))
            .change_byte_order(ByteOrder::LittleEndian);
        assert_eq!(s.deserialize_array(), Ok([0xAF, 0xDE, 0xED]));
    }

    #[test]
    fn deserialize_slice() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAF, 0xDE, 0xED]))
            .change_byte_order(ByteOrder::LittleEndian);
        let mut slc = [0u8, 0u8, 0u8];
        assert_eq!(s.deserialize_slice(&mut slc), Ok(()));
        assert_eq!(slc, [0xAF, 0xDE, 0xED]);
    }

    //--------------------------------------------------------------------------
    // Composites
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_composite() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xAA, 0xBB, 0xFF]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u8(), Ok(0xEE));
        assert_eq!(s.deserialize_composite(|s| { s.deserialize_u16() }), Ok(0xAABB));
        assert_eq!(s.deserialize_u8(), Ok(0xFF));
    }

    //--------------------------------------------------------------------------
    // Byte order
    //--------------------------------------------------------------------------
    #[test]
    fn change_byte_order() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u16(), Ok(0xEEFF));
        assert_eq!(s.with_byte_order(ByteOrder::LittleEndian, |s| s.deserialize_u16()), Ok(0xAABB));
        assert_eq!(s.deserialize_u16(), Ok(0xFFEE));
    }

    //--------------------------------------------------------------------------
    // Deserialize bounded
    //--------------------------------------------------------------------------
    #[test]
    fn deserialize_bounded_eof() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_bounded(1, |de| de.deserialize_u16()), Err(ErrorKind::OutOfBounds.into()));
    }

    #[test]
    fn deserialize_bounded_exact() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_bounded(2, |de| de.deserialize_u16()), Ok(0xEEFF));
        assert_eq!(s.deserialize_u16(), Ok(0xBBAA));
    }

    #[test]
    fn deserialize_bounded_more() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0xFF, 0xBB, 0xAA, 0xFF, 0xEE]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_bounded(3, |de| de.deserialize_u16()), Ok(0xEEFF));
        assert_eq!(s.deserialize_u16(), Ok(0xBBAA));
    }

    //--------------------------------------------------------------------------
    // Padding
    //--------------------------------------------------------------------------
    #[test]
    fn pad_top_level() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xEE, 0x00, 0x00, 0x00]))
            .change_byte_order(ByteOrder::BigEndian);
        assert_eq!(s.deserialize_u8(), Ok(0xEE));
        assert_eq!(s.pad(4), Ok(()));
        assert_eq!(s.take().stream_position(), Ok(4));
    }

    #[test]
    fn pad_length_exceeds_padding() -> Result<(), Error> {
        let mut s =
            StreamDeserializer::new(FixedMemoryStream::new([0xAA, 0xBB, 0xCC])).change_byte_order(ByteOrder::BigEndian);
        s.deserialize_array::<3>()?;
        assert_eq!(s.pad(2), Err(ErrorKind::LengthExceedsPadding.into()));
        Ok(())
    }

    #[test]
    fn pad_composite() {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new([0xAA, 0xBB, 0xCC, 0x01, 0x00, 0x00, 0x00, 0xAF]))
            .change_byte_order(ByteOrder::BigEndian);
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
