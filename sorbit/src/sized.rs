//! The size of serializable types.

/// The size of the type when serialized.
///
/// The size is always the same and known at compilation time.
pub trait ConstSized {
    /// The size of the type in bytes when serialized.
    const SIZE: usize;
}

/// Returns the size of the type in its serialized form.
pub const fn serialized_size_of<T: ConstSized>() -> usize {
    T::SIZE
}

/// Returns the size of the value in its serialized form.
pub const fn serialized_size_of_val<T: ConstSized>(_: &T) -> usize {
    T::SIZE
}

macro_rules! impl_const_sized_primitive {
    ($type:ty) => {
        impl ConstSized for $type {
            const SIZE: usize = ::core::mem::size_of::<$type>();
        }
    };
}

impl_const_sized_primitive!(bool);
impl_const_sized_primitive!(u8);
impl_const_sized_primitive!(u16);
impl_const_sized_primitive!(u32);
impl_const_sized_primitive!(u64);
impl_const_sized_primitive!(u128);
impl_const_sized_primitive!(i8);
impl_const_sized_primitive!(i16);
impl_const_sized_primitive!(i32);
impl_const_sized_primitive!(i64);
impl_const_sized_primitive!(i128);

impl<T: ConstSized, const N: usize> ConstSized for [T; N] {
    const SIZE: usize = T::SIZE * N;
}
