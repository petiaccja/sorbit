use crate::ser_de::{Deserialize, Deserializer, MultiPassSerialize, RevisableSerializer, Serialize, Serializer};

// The normal and multi-pass serializers here are not complete. There should be
// an implementation for every combination, like (S, S), (S, M), (M, S), and
// (M, M). This is pretty much impossible with Rust's current generics.
macro_rules! impl_tuple {
    ($($members:ident),*) => {
        impl<$($members,)*> Serialize for ($($members,)*)
            where $($members: Serialize),*
        {
            fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
                serializer.serialize_composite(|serializer| {
                    #[allow(nonstandard_style)]
                    let ($($members,)*) = self;
                    $($members.serialize(serializer)?;)*
                    serializer.success()
                }).map(|(span, _)| span)
            }
        }

        impl<$($members,)*> MultiPassSerialize for ($($members,)*)
            where $($members: MultiPassSerialize),*
        {
            fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
                serializer.serialize_composite(|serializer| {
                    #[allow(nonstandard_style)]
                    let ($($members,)*) = self;
                    $($members.serialize(serializer)?;)*
                    serializer.success()
                }).map(|(span, _)| span)
            }
        }

        impl<$($members,)*> Deserialize for ($($members,)*)
            where $($members: Deserialize),*
        {
            fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
                Ok((
                    $($members::deserialize(deserializer)?,)*
                ))
            }
        }
    };
}

impl_tuple!(T1);
impl_tuple!(T1, T2);
impl_tuple!(T1, T2, T3);
impl_tuple!(T1, T2, T3, T4);
impl_tuple!(T1, T2, T3, T4, T5);
impl_tuple!(T1, T2, T3, T4, T5, T6);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

#[cfg(test)]
mod tests {
    use crate::ser_de::{FromBytes, ToBytes};

    #[test]
    pub fn serialize_tuple() {
        let value = (0xAB_u8, 0xCDEF_u16);
        let bytes = [0xAB, 0xCD, 0xEF];
        assert_eq!(ToBytes::to_be_bytes(&value).unwrap(), bytes);
        assert_eq!(<(u8, u16)>::from_be_bytes(&bytes).unwrap(), value);
    }
}
