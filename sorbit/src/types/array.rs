use core::mem::MaybeUninit;

use crate::ser_de::{Deserialize, Deserializer, MultiPassSerialize, RevisableSerializer, Serialize, Serializer};

impl<T, const N: usize> Serialize for [T; N]
where
    T: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        // TODO: specialize this for [u8; N] when specialization is available in stable.
        serializer
            .serialize_composite(|serializer| {
                for value in &self[0..(N - 1)] {
                    value.serialize(serializer)?;
                }
                self[N - 1].serialize(serializer)
            })
            .map(|(span, _)| span)
    }
}

impl<T, const N: usize> MultiPassSerialize for [T; N]
where
    T: MultiPassSerialize,
{
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        // TODO: specialize this for [u8; N] when specialization is available in stable.
        serializer
            .serialize_composite(|serializer| {
                for value in &self[0..(N - 1)] {
                    value.serialize(serializer)?;
                }
                self[N - 1].serialize(serializer)
            })
            .map(|(span, _)| span)
    }
}

impl<T, const N: usize> Deserialize for [T; N]
where
    T: Deserialize,
{
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        // TODO: specialize this for [u8; N] when specialization is available in stable.
        // TODO: use core::array::try_from_fn when available in stable.
        let mut array = [const { MaybeUninit::<T>::uninit() }; N];
        for last_idx in 0..N {
            match T::deserialize(deserializer) {
                Ok(value) => array[last_idx].write(value),
                Err(err) => {
                    for inited_idx in 0..last_idx {
                        unsafe { array[inited_idx].assume_init_drop() };
                    }
                    return Err(err);
                }
            };
        }
        Ok(array.map(|maybe_uninit| unsafe { maybe_uninit.assume_init() }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicIsize, Ordering};

    use crate::ser_de::{FromBytes, ToBytes};

    use super::*;

    thread_local! {
        static NUM_CONSTRUCTED: AtomicIsize = AtomicIsize::new(0);
    }

    mod instrumented {

        use super::*;

        #[derive(Debug, PartialEq, Eq)]
        pub struct Instrumented(u8);

        impl Instrumented {
            pub fn new(value: u8) -> Self {
                NUM_CONSTRUCTED.with(|x| x.fetch_add(1, Ordering::Relaxed));
                Self(value)
            }
        }

        impl Serialize for Instrumented {
            fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
                self.0.serialize(serializer)
            }
        }

        impl Deserialize for Instrumented {
            fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
                u8::deserialize(deserializer).map(|x| Self::new(x))
            }
        }

        impl Drop for Instrumented {
            fn drop(&mut self) {
                NUM_CONSTRUCTED.with(|x| x.fetch_sub(1, Ordering::Relaxed));
            }
        }
    }

    use instrumented::Instrumented;

    #[test]
    fn serialize() {
        assert_eq!(NUM_CONSTRUCTED.with(|x| x.load(Ordering::Relaxed)), 0);
        {
            let value = [Instrumented::new(1), Instrumented::new(2)];
            let bytes = [1, 2];
            assert_eq!(value.to_bytes().unwrap(), bytes);
        }
        assert_eq!(NUM_CONSTRUCTED.with(|x| x.load(Ordering::Relaxed)), 0);
    }

    #[test]
    fn deserialize_success() {
        assert_eq!(NUM_CONSTRUCTED.with(|x| x.load(Ordering::Relaxed)), 0);
        {
            let value = [Instrumented::new(1), Instrumented::new(2)];
            let bytes = [1, 2];
            assert_eq!(<[Instrumented; 2]>::from_bytes(&bytes).unwrap(), value);
        }
        assert_eq!(NUM_CONSTRUCTED.with(|x| x.load(Ordering::Relaxed)), 0);
    }

    #[test]
    fn deserialize_failure() {
        assert_eq!(NUM_CONSTRUCTED.with(|x| x.load(Ordering::Relaxed)), 0);
        {
            let bytes = [1];
            assert!(<[Instrumented; 2]>::from_bytes(&bytes).is_err());
        }
        assert_eq!(NUM_CONSTRUCTED.with(|x| x.load(Ordering::Relaxed)), 0);
    }
}
