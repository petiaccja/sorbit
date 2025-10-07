use crate::deserialize::{Deserialize, Deserializer};
use crate::serialize::{Serialize, Serializer};

macro_rules! impl_serialize {
    ($type:ty, $func:ident) => {
        impl Serialize for $type {
            fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
                serializer.$func(*self)
            }
        }
    };
}

macro_rules! impl_deserialize {
    ($type:ty, $func:ident) => {
        impl Deserialize for $type {
            fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
                deserializer.$func()
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

impl_deserialize!(u8, deserialize_u8);
impl_deserialize!(u16, deserialize_u16);
impl_deserialize!(u32, deserialize_u32);
impl_deserialize!(u64, deserialize_u64);
impl_deserialize!(i8, deserialize_i8);
impl_deserialize!(i16, deserialize_i16);
impl_deserialize!(i32, deserialize_i32);
impl_deserialize!(i64, deserialize_i64);
