use crate::ser_de::{Deserialize, Deserializer, Serialize, Serializer};

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
impl_serialize!(u128, serialize_u128);
impl_serialize!(i8, serialize_i8);
impl_serialize!(i16, serialize_i16);
impl_serialize!(i32, serialize_i32);
impl_serialize!(i64, serialize_i64);
impl_serialize!(i128, serialize_i128);

impl_deserialize!(u8, deserialize_u8);
impl_deserialize!(u16, deserialize_u16);
impl_deserialize!(u32, deserialize_u32);
impl_deserialize!(u64, deserialize_u64);
impl_deserialize!(u128, deserialize_u128);
impl_deserialize!(i8, deserialize_i8);
impl_deserialize!(i16, deserialize_i16);
impl_deserialize!(i32, deserialize_i32);
impl_deserialize!(i64, deserialize_i64);
impl_deserialize!(i128, deserialize_i128);

impl Serialize for isize {
    /// `isize` is serialized as its original size. The serialized data is not
    /// sharable between different platforms.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        match size_of::<Self>() {
            1 => (*self as i8).serialize(serializer),
            2 => (*self as i16).serialize(serializer),
            4 => (*self as i32).serialize(serializer),
            8 => (*self as i64).serialize(serializer),
            16 => (*self as i128).serialize(serializer),
            x => panic!("size_of::<isize>() == {x}, can not find equivalent fixed-size integer type"),
        }
    }
}

impl Deserialize for isize {
    /// `isize` is serialized as its original size. The serialized data is not
    /// sharable between different platforms.
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        match size_of::<Self>() {
            1 => i8::deserialize(deserializer).map(|x| x as Self),
            2 => i16::deserialize(deserializer).map(|x| x as Self),
            4 => i32::deserialize(deserializer).map(|x| x as Self),
            8 => i64::deserialize(deserializer).map(|x| x as Self),
            16 => i128::deserialize(deserializer).map(|x| x as Self),
            x => panic!("size_of::<isize>() == {x}, can not find equivalent fixed-size integer type"),
        }
    }
}

impl Serialize for usize {
    /// `usize` is serialized as its original size. The serialized data is not
    /// sharable between different platforms.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        match size_of::<Self>() {
            1 => (*self as u8).serialize(serializer),
            2 => (*self as u16).serialize(serializer),
            4 => (*self as u32).serialize(serializer),
            8 => (*self as u64).serialize(serializer),
            16 => (*self as u128).serialize(serializer),
            x => panic!("size_of::<isize>() == {x}, can not find equivalent fixed-size integer type"),
        }
    }
}

impl Deserialize for usize {
    /// `usize` is serialized as its original size. The serialized data is not
    /// sharable between different platforms.
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        match size_of::<Self>() {
            1 => u8::deserialize(deserializer).map(|x| x as Self),
            2 => u16::deserialize(deserializer).map(|x| x as Self),
            4 => u32::deserialize(deserializer).map(|x| x as Self),
            8 => u64::deserialize(deserializer).map(|x| x as Self),
            16 => u128::deserialize(deserializer).map(|x| x as Self),
            x => panic!("size_of::<isize>() == {x}, can not find equivalent fixed-size integer type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ser_de::{FromBytes, ToBytes};

    use rstest::rstest;

    #[rstest]
    #[case(0x58)]
    #[case(isize::MIN)]
    #[case(isize::MAX)]
    pub fn serialize_isize(#[case] value: isize) {
        let bytes = value.to_be_bytes();
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), bytes);
        assert_eq!(<isize as FromBytes>::from_be_bytes(&bytes).unwrap(), value);
    }

    #[rstest]
    #[case(0x58)]
    #[case(usize::MIN)]
    #[case(usize::MAX)]
    pub fn serialize_usize(#[case] value: usize) {
        let bytes = value.to_be_bytes();
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), bytes);
        assert_eq!(<usize as FromBytes>::from_be_bytes(&bytes).unwrap(), value);
    }
}
