use crate::bit_util::{bit_size_of, bit_size_of_val, keep_lowest_n_bits, zero_lowest_n_bits};

/// Convert a type to an arbitrary bit width representation.
///
/// **Generic parameters**:
/// - `Packed`: the type of the object that holds the arbitrary bit width
///    representation. Typically an unsigned integer, but can be anything.
///
/// This trait is implemented to pack `bool`, signed, and unsigned integers
/// into unsigned integers.
///
/// For example, you create a 5-bit representation of an [`i8`]:
/// ```
/// use serde_binfmt::bit_pack::BitPack;
/// let value: i8 = -7; // 1111_1001
/// let packed : u32 = value.pack(5).unwrap(); // ...0..._11001
/// assert_eq!(packed, 0b11001);
/// ```
///
/// The arbitrary-width packed values can be used to create bit fields.
pub trait BitPack<Packed>
where
    Self: Sized,
{
    /// Pack `self` into a `num_bits`-bit representation.
    ///
    /// The arbtirary-width representation should be placed into
    /// the lowest bits of the output.
    ///
    /// [`None`] is returned when casting `self` to the arbitrary-width
    /// representation would lead to a loss of precision or when the type
    /// that holds the arbitrary is not wide enough to fit the bits.
    fn pack(&self, num_bits: usize) -> Option<Packed>;
}

/// Restore a type from an arbitrary bit width representation.
///
/// **Generic parameters**:
/// - `Packed`: the type of the object that holds the arbitrary bit width
///    representation. Typically an unsigned integer, but can be anything.
///
/// This trait is implemented to unpack `bool`, signed, and unsigned integers
/// from unsigned integers.
///
/// For example, you restore a 5-bit representation of an [`i8`]:
/// ```
/// use serde_binfmt::bit_pack::BitUnpack;
/// let packed : u32 = 0b_11001; // -7 in 5-bit two's complement.
/// let value = i8::unpack(packed, 5).unwrap();
/// assert_eq!(value, 0b_1111_1001_u8.cast_signed()); // -7 in 8-bit two's complement.
/// assert_eq!(value, -7);
/// ```
///
/// You can decode a bit field by repeatedly unpacking different parts of it.
pub trait BitUnpack<Packed>
where
    Self: Sized,
    Packed: Sized,
{
    /// Restore `Self` from a `num_bits`-bit representation.
    ///
    /// The arbtirary-width representation should reside in the lowest bits of
    /// `value`.
    ///
    /// If `Self` cannot hold the value due to numerical precision loss, the
    /// arbitrary-width representation is returned as an error. Attempting to
    /// unpack signed integers when `num_bits` is wider than the `Packed` type
    /// will result in an error too.
    fn unpack(value: Packed, num_bits: usize) -> Result<Self, Packed>;
}

macro_rules! impl_bit_pack_unsigned {
    ($self_ty:ty, $packed_ty:ty) => {
        impl BitPack<$packed_ty> for $self_ty {
            fn pack(&self, num_bits: usize) -> Option<$packed_ty> {
                let casted: $packed_ty = (*self).try_into().ok()?;
                let masked = keep_lowest_n_bits!(casted, num_bits);
                (masked == casted).then_some(masked)
            }
        }
        impl BitUnpack<$packed_ty> for $self_ty {
            fn unpack(value: $packed_ty, num_bits: usize) -> Result<Self, $packed_ty> {
                let masked = keep_lowest_n_bits!(value, num_bits);
                masked.try_into().map_err(|_| value)
            }
        }
    };
}

macro_rules! impl_bit_pack_signed {
    ($self_ty:ty, $packed_ty:ty) => {
        impl BitPack<$packed_ty> for $self_ty {
            fn pack(&self, num_bits: usize) -> Option<$packed_ty> {
                use core::cmp::{max, min};
                let num_value_bits = min(bit_size_of::<$packed_ty>(), min(bit_size_of::<Self>(), max(1, num_bits))) - 1;
                let value = self.cast_unsigned();
                let leading_bits = zero_lowest_n_bits!(value, num_value_bits);
                let are_leading_bits_zeros = leading_bits == 0;
                let are_leading_bits_ones = zero_lowest_n_bits!(!leading_bits, num_bits) == 0;
                let is_negative = *self < 0;
                if are_leading_bits_zeros && !is_negative || are_leading_bits_ones && is_negative {
                    // If leading bits all zeros (000x_xxxx) -> leaves value unchanged (000x_xxxx).
                    // If leading bits all ones (111x_xxxx)-> zeroes out leading bits except sign bit (001x_xxxx).
                    let masked = value ^ (leading_bits << 1);
                    let padding_pattern: $packed_ty = if is_negative { !0 } else { 0 };
                    let padding = keep_lowest_n_bits!(zero_lowest_n_bits!(padding_pattern, num_value_bits), num_bits);
                    masked.pack(num_bits).map(|packed: $packed_ty| packed | padding)
                } else {
                    None
                }
            }
        }
        impl BitUnpack<$packed_ty> for $self_ty {
            fn unpack(packed: $packed_ty, num_bits: usize) -> Result<Self, $packed_ty> {
                use core::cmp::max;
                let truncated = <$packed_ty>::unpack(packed, num_bits)?;
                if num_bits <= bit_size_of_val(&truncated) {
                    let is_negative = 1 == 1 & (truncated >> (max(1, num_bits) - 1));
                    let padding_pattern: $packed_ty = if is_negative { !0 } else { 0 };
                    let padding = zero_lowest_n_bits!(padding_pattern, num_bits);
                    (padding | truncated).cast_signed().try_into().map_err(|_| packed)
                } else {
                    Err(packed)
                }
            }
        }
    };
}

macro_rules! impl_bit_pack_bool {
    ($packed_ty:ty) => {
        impl BitPack<$packed_ty> for bool {
            fn pack(&self, num_bits: usize) -> Option<$packed_ty> {
                (num_bits > 0).then_some(self.clone().into())
            }
        }
        impl BitUnpack<$packed_ty> for bool {
            fn unpack(value: $packed_ty, num_bits: usize) -> Result<Self, $packed_ty> {
                let masked = keep_lowest_n_bits!(value, num_bits);
                match masked {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => Err(value),
                }
            }
        }
    };
}

impl_bit_pack_unsigned!(u8, u8);
impl_bit_pack_unsigned!(u8, u16);
impl_bit_pack_unsigned!(u8, u32);
impl_bit_pack_unsigned!(u8, u64);

impl_bit_pack_unsigned!(u16, u8);
impl_bit_pack_unsigned!(u16, u16);
impl_bit_pack_unsigned!(u16, u32);
impl_bit_pack_unsigned!(u16, u64);

impl_bit_pack_unsigned!(u32, u8);
impl_bit_pack_unsigned!(u32, u16);
impl_bit_pack_unsigned!(u32, u32);
impl_bit_pack_unsigned!(u32, u64);

impl_bit_pack_unsigned!(u64, u8);
impl_bit_pack_unsigned!(u64, u16);
impl_bit_pack_unsigned!(u64, u32);
impl_bit_pack_unsigned!(u64, u64);

impl_bit_pack_signed!(i8, u8);
impl_bit_pack_signed!(i8, u16);
impl_bit_pack_signed!(i8, u32);
impl_bit_pack_signed!(i8, u64);

impl_bit_pack_signed!(i16, u8);
impl_bit_pack_signed!(i16, u16);
impl_bit_pack_signed!(i16, u32);
impl_bit_pack_signed!(i16, u64);

impl_bit_pack_signed!(i32, u8);
impl_bit_pack_signed!(i32, u16);
impl_bit_pack_signed!(i32, u32);
impl_bit_pack_signed!(i32, u64);

impl_bit_pack_signed!(i64, u8);
impl_bit_pack_signed!(i64, u16);
impl_bit_pack_signed!(i64, u32);
impl_bit_pack_signed!(i64, u64);

impl_bit_pack_bool!(u8);
impl_bit_pack_bool!(u16);
impl_bit_pack_bool!(u32);
impl_bit_pack_bool!(u64);

#[cfg(test)]
mod tests {
    use super::*;

    //--------------------------------------------------------------------------
    // Pack unsigned.
    //--------------------------------------------------------------------------
    #[test]
    fn pack_unsigned_into_narrower_success() {
        let value: u16 = 0b0000_0000_0001_0000;
        let expected: u8 = 0b0001_0000;
        assert_eq!(value.pack(6), Some(expected));
    }

    #[test]
    fn pack_unsigned_into_narrower_overflow_type() {
        let value: u16 = 0b0000_0001_0000_0000;
        assert_eq!(value.pack(16), Option::<u8>::None);
    }

    #[test]
    fn pack_unsigned_into_narrower_overflow_value() {
        let value: u16 = 0b0000_0000_0001_0000;
        assert_eq!(value.pack(4), Option::<u8>::None);
    }

    #[test]
    fn pack_unsigned_into_narrower_overshift() {
        let value: u16 = 0b0000_0000_0001_0000;
        let expected: u8 = 0b0001_0000;
        assert_eq!(value.pack(73), Some(expected));
    }

    #[test]
    fn pack_unsigned_into_wider_success() {
        let value: u8 = 0b0001_0000;
        let expected: u16 = 0b0000_0000_0001_0000;
        assert_eq!(value.pack(6), Some(expected));
    }

    #[test]
    fn pack_unsigned_into_wider_overflow_value() {
        let value: u16 = 0b0000_0000_0001_0000;
        assert_eq!(value.pack(4), Option::<u8>::None);
    }

    #[test]
    fn pack_unsigned_into_wider_overshift() {
        let value: u8 = 0b0001_0000;
        let expected: u16 = 0b0000_0000_0001_0000;
        assert_eq!(value.pack(73), Some(expected));
    }

    //--------------------------------------------------------------------------
    // Unpack unsigned.
    //--------------------------------------------------------------------------

    #[test]
    fn unpack_unsigned_from_wider_success() {
        let packed: u16 = 0b0000_0000_0001_0000;
        let expected: u8 = 0b0001_0000;
        assert_eq!(u8::unpack(packed, 6), Ok(expected));
    }

    #[test]
    fn unpack_unsigned_from_wider_overflow_type() {
        let packed: u16 = 0b0000_0001_0000_0000;
        assert_eq!(u8::unpack(packed, 16), Err(packed));
    }

    #[test]
    fn unpack_unsigned_from_wider_dirty_high() {
        let packed: u16 = 0b0000_0000_0010_1000;
        let expected: u8 = 0b1000;
        assert_eq!(u8::unpack(packed, 4), Ok(expected));
    }

    #[test]
    fn unpack_unsigned_from_wider_overshift() {
        let packed: u16 = 0b0000_0000_0001_0000;
        let expected: u8 = 0b0001_0000;
        assert_eq!(u8::unpack(packed, 73), Ok(expected));
    }

    #[test]
    fn unpack_unsigned_from_narrower_success() {
        let packed: u8 = 0b0001_0000;
        let expected: u16 = 0b0000_0000_0001_0000;
        assert_eq!(u16::unpack(packed, 6), Ok(expected));
    }

    #[test]
    fn unpack_unsigned_from_narrower_dirty_high() {
        let packed: u8 = 0b0010_1000;
        let expected: u16 = 0b0000_0000_0000_1000;
        assert_eq!(u16::unpack(packed, 4), Ok(expected));
    }

    #[test]
    fn unpack_unsigned_from_narrower_overshift() {
        let packed: u8 = 0b0001_0000;
        let expected: u16 = 0b0000_0000_0001_0000;
        assert_eq!(u16::unpack(packed, 73), Ok(expected));
    }

    //--------------------------------------------------------------------------
    // Pack signed.
    //--------------------------------------------------------------------------
    #[test]
    fn pack_signed_into_narrower_success() {
        {
            let value: i16 = 10;
            let expected: u8 = 10;
            assert_eq!(value.pack(5), Some(expected));
        }
        {
            let value: i16 = -10;
            let expected = (-10i8).cast_unsigned() & 0b0001_1111;
            assert_eq!(value.pack(5), Some(expected));
        }
        {
            let value: i16 = 127;
            let expected: u8 = 127;
            assert_eq!(value.pack(8), Some(expected));
        }
        {
            let value: i16 = -128;
            let expected = (-128i8).cast_unsigned() & 0b1111_1111;
            assert_eq!(value.pack(8), Some(expected));
        }
    }

    #[test]
    fn pack_signed_into_narrower_overflow_type() {
        {
            let value: i16 = 128;
            assert_eq!(value.pack(16), Option::<u8>::None);
        }
        {
            let value: i16 = -129;
            assert_eq!(value.pack(16), Option::<u8>::None);
        }
    }

    #[test]
    fn pack_signed_into_narrower_overflow_value() {
        {
            let value: i16 = 16;
            assert_eq!(value.pack(5), Option::<u8>::None);
        }
        {
            let value: i16 = -17;
            assert_eq!(value.pack(5), Option::<u8>::None);
        }
    }

    #[test]
    fn pack_signed_into_narrower_overshift() {
        {
            let value: i16 = 10;
            let expected: u8 = 10;
            assert_eq!(value.pack(73), Some(expected));
        }
        {
            let value: i16 = -10;
            let expected = (-10i8).cast_unsigned();
            assert_eq!(value.pack(73), Some(expected));
        }
    }

    #[test]
    fn pack_signed_into_wider_success() {
        {
            let value: i8 = 15;
            let expected: u16 = 15;
            assert_eq!(value.pack(5), Some(expected));
        }
        {
            let value: i8 = -16;
            let expected = (-16i16).cast_unsigned() & 0b0001_1111;
            assert_eq!(value.pack(5), Some(expected));
        }
    }

    #[test]
    fn pack_signed_into_wider_overflow_value() {
        {
            let value: i8 = 16;
            assert_eq!(value.pack(5), Option::<u16>::None);
        }
        {
            let value: i8 = -17;
            assert_eq!(value.pack(5), Option::<u16>::None);
        }
    }

    #[test]
    fn pack_signed_into_wider_overshift() {
        {
            let value: i8 = 10;
            let expected: u16 = 10;
            assert_eq!(value.pack(73), Some(expected));
        }
        {
            let value: i8 = -10;
            let expected = (-10i16).cast_unsigned();
            assert_eq!(value.pack(73), Some(expected));
        }
    }

    //--------------------------------------------------------------------------
    // Unpack signed.
    //--------------------------------------------------------------------------

    #[test]
    fn unpack_signed_from_wider_success() {
        {
            let packed: u16 = 6;
            let expected: i8 = 6;
            assert_eq!(i8::unpack(packed, 6), Ok(expected));
        }
        {
            let packed: u16 = (-6i16).cast_unsigned() & 0b0011_1111;
            let expected: i8 = -6;
            assert_eq!(i8::unpack(packed, 6), Ok(expected));
        }
        {
            let packed: u16 = 15;
            let expected: i8 = 15;
            assert_eq!(i8::unpack(packed, 5), Ok(expected));
        }
        {
            let packed: u16 = (-16i16).cast_unsigned() & 0b0001_1111;
            let expected: i8 = -16;
            assert_eq!(i8::unpack(packed, 5), Ok(expected));
        }
    }

    #[test]
    fn unpack_signed_from_wider_overflow_type() {
        {
            let packed: u16 = 128;
            assert_eq!(i8::unpack(packed, 16), Err(packed));
        }
        {
            let packed: u16 = (-129i16).cast_unsigned();
            assert_eq!(i8::unpack(packed, 16), Err(packed));
        }
    }

    #[test]
    fn unpack_signed_from_wider_dirty_high() {
        {
            let packed: u16 = 10 | 0b0100_1001_1000_0000;
            let expected: i8 = 10;
            assert_eq!(i8::unpack(packed, 5), Ok(expected));
        }
        {
            let packed: u16 = (-10i16).cast_unsigned() & 0b0100_1001_1001_1111;
            let expected: i8 = -10;
            assert_eq!(i8::unpack(packed, 5), Ok(expected));
        }
    }

    #[test]
    fn unpack_signed_from_wider_overshift() {
        // Unliked unsigned with overshift, this has to fail.
        // With unsigned integers, we can fill in zeros into the higher bits.
        // With signed, we have to know the sign bit, and we cannot assume zero.
        {
            let packed: u16 = 10;
            assert_eq!(i8::unpack(packed, 73), Err(packed));
        }
        {
            let packed: u16 = (-10i16).cast_unsigned();
            assert_eq!(i8::unpack(packed, 73), Err(packed));
        }
    }

    #[test]
    fn unpack_signed_from_narrower_success() {
        {
            let packed: u8 = 10;
            let expected: i16 = 10;
            assert_eq!(i16::unpack(packed, 5), Ok(expected));
        }
        {
            let packed: u8 = (-10i8).cast_unsigned();
            let expected: i16 = -10;
            assert_eq!(i16::unpack(packed, 5), Ok(expected));
        }
    }

    #[test]
    fn unpack_signed_from_narrower_dirty_high() {
        {
            let packed: u8 = 10 | 0b1000_0000;
            let expected: i16 = 10;
            assert_eq!(i16::unpack(packed, 5), Ok(expected));
        }
        {
            let packed: u8 = (-10i8).cast_unsigned() & 0b1001_1111;
            let expected: i16 = -10;
            assert_eq!(i16::unpack(packed, 5), Ok(expected));
        }
    }

    #[test]
    fn unpack_signed_from_narrower_overshift() {
        // Unlike unsigned, this is expected to fail.
        // See explanation above in from_wider.
        {
            let packed: u8 = 10;
            assert_eq!(i16::unpack(packed, 73), Err(packed));
        }
        {
            let packed: u8 = (-10i8).cast_unsigned();
            assert_eq!(i16::unpack(packed, 73), Err(packed));
        }
    }

    //--------------------------------------------------------------------------
    // Pack & unpack bool.
    //--------------------------------------------------------------------------

    #[test]
    fn pack_bool() {
        assert_eq!(false.pack(2), Some(0u8));
        assert_eq!(true.pack(2), Some(1u8));
    }

    #[test]
    fn pack_bool_zero_bits() {
        assert_eq!(false.pack(0), Option::<u8>::None);
        assert_eq!(true.pack(0), Option::<u8>::None);
    }

    #[test]
    fn unpack_bool() {
        assert_eq!(bool::unpack(0u8, 2), Ok(false));
        assert_eq!(bool::unpack(1u8, 2), Ok(true));
        assert_eq!(bool::unpack(3u8, 2), Err(3));
    }

    #[test]
    fn unpack_zero_bits() {
        assert_eq!(bool::unpack(0u8, 0), Ok(false));
        assert_eq!(bool::unpack(1u8, 0), Ok(false));
        assert_eq!(bool::unpack(3u8, 0), Ok(false));
    }
}
