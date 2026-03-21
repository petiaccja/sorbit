use std::marker::PhantomData;

use crate::ser_de::{Deserialize, Serialize};

impl<T> Serialize for PhantomData<T> {
    fn serialize<S: crate::ser_de::Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer.success()
    }
}

impl<T> Deserialize for PhantomData<T> {
    fn deserialize<D: crate::ser_de::Deserializer>(_deserializer: &mut D) -> Result<Self, D::Error> {
        Ok(PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use crate::ser_de::{FromBytes as _, ToBytes as _};

    #[test]
    fn serialize() {
        assert_eq!(PhantomData::<u8>.to_bytes(), Ok(vec![]));
    }

    #[test]
    fn deserialize() {
        assert_eq!(PhantomData::<u8>::from_bytes(&[]), Ok(PhantomData));
    }
}
