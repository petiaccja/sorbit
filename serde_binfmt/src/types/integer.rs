use crate::serialize::Serialize;
use crate::serialize::Serializer;

macro_rules! impl_serialize {
    ($type:ty, $func:ident) => {
        impl Serialize for $type {
            fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
                serializer.$func(*self)
            }
        }
    };
}

impl_serialize!(u8, serialize_u8);
impl_serialize!(u16, serialize_u16);
impl_serialize!(u32, serialize_u32);
impl_serialize!(u64, serialize_u64);
impl_serialize!(i8, serialize_i8);
impl_serialize!(i16, serialize_i16);
impl_serialize!(i32, serialize_i32);
impl_serialize!(i64, serialize_i64);
