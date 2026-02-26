/// Returns the size of the type in bits.
///
/// This is equivalent to `8 * size_of::<T>()`.
pub const fn bit_size_of<T: Sized>() -> usize {
    8 * size_of::<T>()
}

/// Returns the size of the pointed-to value in bits.
///
/// This is equivalent to `8 * size_of_val(val)`.
pub const fn bit_size_of_val<T: Sized>(val: &T) -> usize {
    8 * size_of_val(val)
}

macro_rules! keep_lowest_n_bits {
    ($value:expr, $n:expr) => {
        $value & !(!($value ^ $value)).unbounded_shl($n as u32)
    };
}

macro_rules! zero_lowest_n_bits {
    ($value:expr, $n:expr) => {
        $value & (!($value ^ $value)).unbounded_shl($n as u32)
    };
}

pub(crate) use keep_lowest_n_bits;
pub(crate) use zero_lowest_n_bits;
