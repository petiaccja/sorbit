use core::ops::{Add, BitOrAssign, Bound::*, Range, RangeBounds};
use num::PrimInt;

use crate::bit::Error;

use super::bit_pack::{PackInto, UnpackFrom};
use super::bit_util::{bit_size_of, keep_lowest_n_bits};

/// A bit field whose members are defined at runtime.
///
/// Unlike traditional bit fields that define a fixed set of members with
/// their bit width and offset at compile time, this data structure works
/// entirely at runtime, and can therefore represent any bit field. You can
/// start with an empty bit field via [`Self::new`], and then use [`Self::pack`]
/// to add the members. You can extract a member via [`Self::unpack`].
///
/// To check for overwriting the same bits when adding a new member, the [`BitField`]
/// stores a mask internally. The affected bits of the mask are set to 1 when a
/// new member is added via [`Self::pack`]. The next time the same bits are written,
/// an error is raised.
///
/// # Generic parameters
///
/// - `Packed`: the underlying type of the bit field. For example, when all the
///   members of the bit field span 16 bits together, a `u16` could be used for
///   `Packed`, although anything larger than that, such as a `u32`, would work
///   too.
pub struct BitField<Packed: PrimInt>
where
    Packed: PrimInt + BitOrAssign,
{
    bits: Packed,
    mask: Packed,
}

/// Create a bit field from all of its members in one step.
///
/// # Example
///
/// ```
/// use sorbit::pack_bit_field;
///
/// let bit_field = pack_bit_field!(u16 => {
///     (3u8, 3..6),
///     (4u8, 11..15)
/// })?;
/// assert_eq!(bit_field, 0b0010_0000_0001_1000);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[macro_export]
macro_rules! pack_bit_field {
    ($packed_ty:ty => { $(($value:expr, $target_bits:expr)),*}) => {
        {
            let mut bit_field = ::sorbit::bit::BitField::<$packed_ty>::new();
            let results = [$(bit_field.pack($value, $target_bits),)*];
            if let ::core::option::Option::Some(::core::result::Result::Err(err)) = results.into_iter().find(|result| result.is_err()) {
                Err(err)
            } else {
                ::core::result::Result::Ok(bit_field.into_bits())
            }
        }
    };
}

/// Deconstruct a bit field into its members in a single step.
///
/// # Example
///
/// ```
/// use sorbit::unpack_bit_field;
///
/// let (a, b) = unpack_bit_field!(0b0010_0000_0001_1000u16 => {
///     (u8, 3..6),
///     (u8, 11..15)
/// })?;
/// assert_eq!(a, 3u8);
/// assert_eq!(b, 4u8);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[macro_export]
macro_rules! unpack_bit_field {
    ($bit_field:expr => { $(($member_ty:ty, $source_bits:expr)),*}) => {
        {
            let bit_field = ::sorbit::bit::BitField::from_bits($bit_field);
            move || -> ::core::result::Result<($($member_ty,)*), ::sorbit::bit::Error> {
                ::core::result::Result::Ok(($(bit_field.unpack::<$member_ty, _, _>($source_bits)?,)*))
            }()
        }
    };
}

impl<Packed> BitField<Packed>
where
    Packed: PrimInt + BitOrAssign,
    u64: PackInto<Packed>,
{
    /// Create a new bit field all bits set to zero and the mask set to zero as well.
    pub fn new() -> Self {
        Self { bits: Packed::zero(), mask: Packed::zero() }
    }

    /// Create a new bit field from the given bits, with the mask set to all zeros.
    pub fn from_bits(bits: Packed) -> Self {
        Self { bits, mask: Packed::zero() }
    }

    /// The size of the bit field's underlying type in bits.
    pub fn bit_size_of(&self) -> usize {
        bit_size_of::<Packed>()
    }

    /// Add a new member to the bit field.
    ///
    /// If not all bits of the mask at the target bits are zero, an error is
    /// raised. Otherwise, all bits of the mask are set to one, marking those bits
    /// as occupied, raising an error next to you attempt to write them.
    ///
    /// [`PackInto::pack_into`] is used to first convert `value` into its arbitrary
    /// width representation. (The width of derived from `target_bits`.) Then, the
    /// packed bits are copied into the bit field at `target_bits`.
    ///
    /// # Parameters
    ///
    /// - `value`: the bits of the new member.
    /// - `target_bits`: the bit range where the new member is inserted. The least
    ///   significant bit is numbered zero (LSB0).
    pub fn pack<Value, BitRange, BitScalar>(&mut self, value: Value, target_bits: BitRange) -> Result<(), Error>
    where
        Value: PackInto<Packed>,
        BitRange: RangeBounds<BitScalar>,
        BitScalar: Add + Into<i64> + Clone,
    {
        let to_bits = reduce_range(&target_bits, &Self::space());
        Self::validate_range(&to_bits)?;
        let num_bits = (to_bits.end - to_bits.start) as usize;
        let mask_bits: Packed =
            keep_lowest_n_bits!(!0u64, num_bits).pack_into(num_bits).expect("high bits not cut properly");
        let mask_placed = mask_bits << (to_bits.start as usize);
        if (self.mask | mask_placed).count_ones() != self.mask.count_ones() + num_bits as u32 {
            return Err(Error::Overlap);
        }

        let packed_bits: Packed = value.pack_into(num_bits).ok_or(Error::TooManyBits)?;
        let packed_placed = packed_bits << to_bits.start as usize;

        self.mask |= mask_placed;
        self.bits |= packed_placed;
        Ok(())
    }

    /// Read a member of the bit field.
    ///
    /// This does not check the mask and the member will be read as long as the
    /// source bits do not fall outside the bit field.
    ///
    /// First, the arbitrary width representation is obtained by extracting the bits
    /// from the bit field at `source_bits`. Then, [`UnpackFrom::unpack_from`] is
    /// used to convert the arbitrary width bits to the output type.
    ///
    /// # Parameters
    ///
    /// - `source_bits`: the bit range where the member to read resides. The least
    ///   significant bit is numbered zero (LSB0).
    pub fn unpack<Value, BitRange, BitScalar>(&self, source_bits: BitRange) -> Result<Value, Error>
    where
        Value: UnpackFrom<Packed>,
        BitRange: RangeBounds<BitScalar>,
        BitScalar: Add + Into<i64> + Clone,
    {
        let from_bits = reduce_range(&source_bits, &Self::space());
        Self::validate_range(&from_bits)?;
        let num_bits = (from_bits.end - from_bits.start) as usize;
        Value::unpack_from(self.bits >> from_bits.start as usize, num_bits).map_err(|_| Error::TooManyBits)
    }

    /// Convert the bit field to its underlying type.
    ///
    /// The mask is dropped.
    pub fn into_bits(self) -> Packed {
        self.bits
    }

    const fn space() -> Range<i64> {
        0..(bit_size_of::<Packed>() as i64)
    }

    fn validate_range(range: &Range<i64>) -> Result<(), Error> {
        let space = Self::space();
        let is_start_within_space = space.contains(&range.start);
        let is_end_within_space = space.contains(&(range.end - 1));
        let is_not_reversed = range.start <= range.end;

        let is_start_within_space_err = is_start_within_space.then_some(()).ok_or(Error::OutOfRange);
        let is_end_within_space_err = is_end_within_space.then_some(()).ok_or(Error::OutOfRange);
        let is_not_reversed_err = is_not_reversed.then_some(()).ok_or(Error::ReversedRange);

        is_start_within_space_err.and(is_end_within_space_err).and(is_not_reversed_err)
    }
}

fn reduce_range<BitScalar, BitRange>(range: &BitRange, space: &Range<i64>) -> Range<i64>
where
    BitScalar: Add + Into<i64> + Clone,
    BitRange: RangeBounds<BitScalar>,
{
    match (range.start_bound(), range.end_bound()) {
        (Included(start), Included(end)) => (start.clone().into())..(end.clone().into() + 1),
        (Included(start), Excluded(end)) => (start.clone().into())..(end.clone().into()),
        (Excluded(start), Included(end)) => (start.clone().into() + 1)..(end.clone().into() + 1),
        (Excluded(start), Excluded(end)) => (start.clone().into() + 1)..end.clone().into(),
        (Included(start), Unbounded) => (start.clone().into())..space.end,
        (Excluded(start), Unbounded) => (start.clone().into() + 1)..space.end,
        (Unbounded, Included(end)) => 0..(end.clone().into() + 1),
        (Unbounded, Excluded(end)) => 0..(end.clone().into()),
        (Unbounded, Unbounded) => space.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_multiple() {
        let mut bit_field = BitField::<u32>::new();
        bit_field.pack(0b1011_u8, 7..11).unwrap();
        bit_field.pack(0b11_1011_u8, 18..24).unwrap();
        assert_eq!(bit_field.into_bits(), 0b_0000_0000_1110_1100_0000_0101_1000_0000);
    }

    #[test]
    fn pack_stretch() {
        let mut bit_field = BitField::<u32>::new();
        bit_field.pack(0b1011_u8, 7..31).unwrap();
        assert!(bit_field.pack(0b00_01_u8, 30..31).is_err());
        assert!(bit_field.pack(0b00_01_u8, 31..32).is_ok());
        assert!(bit_field.pack(0b00_01_u8, 7..8).is_err());
        assert!(bit_field.pack(0b00_01_u8, 0..1).is_ok());
        assert_eq!(bit_field.into_bits(), 0b_1000_0000_0000_0000_0000_0101_1000_0001);
    }

    #[test]
    fn pack_overlap() {
        let mut bit_field = BitField::<u32>::new();
        bit_field.pack(0b1011_u8, 7..11).unwrap();
        assert!(bit_field.pack(0b11_1011_u8, 10..16).is_err());
    }

    #[test]
    fn pack_space_overflow() {
        let mut bit_field = BitField::<u32>::new();
        assert!(bit_field.pack(0b1011_u8, 30..34).is_err());
    }

    #[test]
    fn pack_space_underflow() {
        let mut bit_field = BitField::<u32>::new();
        assert!(bit_field.pack(0b1011_u8, -2..2).is_err());
    }

    #[test]
    fn pack_reversed() {
        let mut bit_field = BitField::<u32>::new();
        assert!(bit_field.pack(0b1011_u8, 11..7).is_err());
    }

    #[test]
    fn unpack() {
        let bit_field = BitField::from_bits(0b0000_0101_1000_0001_u16);
        let value: i8 = bit_field.unpack(7..11).unwrap();
        assert_eq!(value, -5);
    }

    #[test]
    fn unpack_space_overflow() {
        let bit_field = BitField::from_bits(0b0000_0101_1000_0001_u16);
        assert!(bit_field.unpack::<u8, _, _>(7..19).is_err());
    }

    #[test]
    fn unpack_space_underflow() {
        let bit_field = BitField::from_bits(0b0000_0101_1000_0001_u16);
        assert!(bit_field.unpack::<u8, _, _>(-2..7).is_err());
    }

    #[test]
    fn unpack_reversed() {
        let bit_field = BitField::from_bits(0b0000_0101_1000_0001_u16);
        assert!(bit_field.unpack::<u8, _, _>(11..7).is_err());
    }

    #[test]
    fn bit_field_macro_one() {
        let value = pack_bit_field!(u8 => { (0b11u8, 0..2) });
        assert_eq!(value, Ok(0b0000_0011));
    }

    #[test]
    fn bit_field_macro_multiple() {
        let value = pack_bit_field!(u8 => { (0b11u8, 0..2), (0b1001u8, 2..6) });
        assert_eq!(value, Ok(0b0010_0111));
    }

    #[test]
    fn unpack_macro_one() {
        let unpacked = unpack_bit_field!(0b0010_0111_u8 => { (u8, 0..2) });
        assert_eq!(unpacked, Ok((0b11u8,)));
    }

    #[test]
    fn unpack_macro_multiple() {
        let unpacked = unpack_bit_field!(0b0010_0111_u8 => { (u8, 0..2), (u8, 2..6) });
        assert_eq!(unpacked, Ok((0b11u8, 0b1001u8)));
    }
}
