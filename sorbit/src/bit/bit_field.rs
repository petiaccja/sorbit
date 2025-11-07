use core::ops::{Add, BitOrAssign, Bound::*, Range, RangeBounds};
use num::PrimInt;

use super::bit_pack::{PackInto, UnpackFrom};
use super::bit_util::{bit_size_of, keep_lowest_n_bits};

pub struct BitField<Packed: PrimInt>
where
    Packed: PrimInt + BitOrAssign,
    u64: PackInto<Packed>,
{
    bits: Packed,
    mask: Packed,
}

#[macro_export]
macro_rules! pack_bit_field {
    ($packed_ty:ty => { $(($value:expr, $to_bits:expr)),*}) => {
        {
            let mut bit_field = ::sorbit::bit::BitField::<$packed_ty>::new();
            let success = [$(bit_field.pack($value, $to_bits).is_ok(),)*];
            if success.iter().all(|s| *s) {
                ::core::result::Result::Ok(bit_field.into_bits())
            } else {
                Err(())
            }
        }
    };
}

#[macro_export]
macro_rules! unpack_bit_field {
    ($bit_field:expr => { $(($self_ty:ty, $to_bits:expr)),*}) => {
        {
            let bit_field = ::sorbit::bit::BitField::from_bits($bit_field);
            move || -> ::core::result::Result<($($self_ty,)*), ()> {
                ::core::result::Result::Ok(($(bit_field.unpack::<$self_ty, _, _>($to_bits).map_err(|_| ())?,)*))
            }()
        }
    };
}

impl<Packed> BitField<Packed>
where
    Packed: PrimInt + BitOrAssign,
    u64: PackInto<Packed>,
{
    pub fn new() -> Self {
        Self { bits: Packed::zero(), mask: Packed::zero() }
    }

    pub fn from_bits(bits: Packed) -> Self {
        Self { bits, mask: Packed::zero() }
    }

    pub fn pack<Value, BitRange, BitScalar>(&mut self, value: Value, to_bits: BitRange) -> Result<(), Value>
    where
        Value: PackInto<Packed>,
        BitRange: RangeBounds<BitScalar>,
        BitScalar: Add + Into<i64> + Clone,
    {
        let to_bits = reduce_range(&to_bits, &Self::space());
        if !Self::is_range_valid(&to_bits) {
            return Err(value);
        }
        let num_bits = (to_bits.end - to_bits.start) as usize;
        let mask_bits: Packed =
            keep_lowest_n_bits!(!0u64, num_bits).pack_into(num_bits).expect("high bits not cut properly");
        let mask_placed = mask_bits << (to_bits.start as usize);
        if (self.mask | mask_placed).count_ones() != self.mask.count_ones() + num_bits as u32 {
            // Error: we are overwriting another packed object in the bit field.
            return Err(value);
        }

        let packed_bits: Packed = value.pack_into(num_bits).ok_or(value)?;
        let packed_placed = packed_bits << to_bits.start as usize;

        self.mask |= mask_placed;
        self.bits |= packed_placed;
        Ok(())
    }

    pub fn unpack<Value, BitRange, BitScalar>(&self, from_bits: BitRange) -> Result<Value, ()>
    where
        Value: UnpackFrom<Packed>,
        BitRange: RangeBounds<BitScalar>,
        BitScalar: Add + Into<i64> + Clone,
    {
        let from_bits = reduce_range(&from_bits, &Self::space());
        if !Self::is_range_valid(&from_bits) {
            // Error: the target bits must be in non-reversed order.
            return Err(());
        }
        let num_bits = (from_bits.end - from_bits.start) as usize;
        Value::unpack_from(self.bits >> from_bits.start as usize, num_bits).map_err(|_| ())
    }

    pub fn into_bits(self) -> Packed {
        self.bits
    }

    const fn space() -> Range<i64> {
        0..(bit_size_of::<Packed>() as i64)
    }

    fn is_range_valid(range: &Range<i64>) -> bool {
        let space = Self::space();
        let is_start_within_space = space.contains(&range.start);
        let is_end_within_space = space.contains(&(range.end - 1));
        let is_not_reversed = range.start <= range.end;
        is_start_within_space && is_end_within_space && is_not_reversed
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
