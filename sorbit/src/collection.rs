//! Utilities for serializing collections, like `Vec`.

use crate::ser_de::{Deserialize, Deserializer, MultiPassSerialize, RevisableSerializer, Serialize, Serializer, Span};

/// Return the length of a collection as a specific (integer) type.
pub trait LenAs<T> {
    /// Return the length of a collection as a specific (integer) type.
    ///
    /// If the length of  can not be represented by `T`, [`None`] is returned.
    fn len_as(&self) -> Option<T>;
}

// Blanket implementation of [`LenAs`] for collections.
impl<T, C> LenAs<T> for C
where
    for<'c> &'c C: IntoIterator<IntoIter: ExactSizeIterator>,
    T: TryFrom<usize>,
{
    fn len_as(&self) -> Option<T> {
        T::try_from(self.into_iter().len()).ok()
    }
}

/// Serialize the items of a collection without serializing its length.
pub trait SerializeItems {
    /// Serialize the items of a collection without serializing its length.
    fn serialize_items<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error>;
}

/// Serialize the items of a collection without serializing its length.
pub trait MultiPassSerializeItems {
    /// Serialize the items of a collection without serializing its length.
    fn serialize_items<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error>;
}

// Blanket implementation of [`SerializeItems`] for collections.
impl<C> SerializeItems for C
where
    for<'c> &'c C: IntoIterator<Item: Serialize>,
{
    fn serialize_items<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer
            .serialize_composite(|serializer| {
                for item in self {
                    item.serialize(serializer)?;
                }
                serializer.success()
            })
            .map(|(composite_span, _)| composite_span)
    }
}

// Blanket implementation of [`MultiPassSerializeItems`] for collections.
impl<C> MultiPassSerializeItems for C
where
    for<'c> &'c C: IntoIterator<Item: MultiPassSerialize>,
{
    fn serialize_items<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer
            .serialize_composite(|serializer| {
                for item in self {
                    item.serialize(serializer)?;
                }
                serializer.success()
            })
            .map(|(composite_span, _)| composite_span)
    }
}

/// Deserialize the collection given the number of its elements is given.
pub trait DeserializeByLen<T, Item> {
    /// Deserialize the collection given the number of its elements is given.
    fn deserialize_by_len<D: Deserializer>(deserializer: &mut D, len: T) -> Result<Self, D::Error>
    where
        Self: Sized;
}

// Blanket implementation of [`MultiPassSerializeItems`] for collections.
impl<T, C, Item> DeserializeByLen<T, Item> for C
where
    Item: Deserialize,
    C: FromIterator<Item>,
    usize: TryFrom<T>,
{
    fn deserialize_by_len<D: Deserializer>(deserializer: &mut D, len: T) -> Result<Self, D::Error>
    where
        Self: Sized,
    {
        let Ok(len) = usize::try_from(len) else {
            return deserializer.error("the length of the collection can not be converted into a `usize`");
        };
        (0..len).into_iter().map(|_| Item::deserialize(deserializer)).collect()
    }
}

/// Deserialize an object given the number of its bytes is given.
pub trait DeserializeByByteCount<T, Item> {
    /// Deserialize an object given the number of its bytes is given.
    fn deserialize_by_byte_count<D: Deserializer>(deserializer: &mut D, byte_count: T) -> Result<Self, D::Error>
    where
        Self: Sized;
}

impl<T, C, Item> DeserializeByByteCount<T, Item> for C
where
    Item: Deserialize,
    C: FromIterator<Item>,
    usize: TryFrom<T>,
{
    fn deserialize_by_byte_count<D: Deserializer>(deserializer: &mut D, byte_count: T) -> Result<Self, D::Error>
    where
        Self: Sized,
    {
        let Ok(byte_count) = usize::try_from(byte_count) else {
            return deserializer.error("the length of the collection can not be converted into a `usize`");
        };
        deserializer.deserialize_bounded(byte_count as u64, |deserializer| {
            (0..)
                .into_iter()
                .map_while(|_| {
                    (0 != deserializer.bytes_in_bounds().expect("expected to be Some within deserialize_bounded"))
                        .then(|| Item::deserialize(deserializer))
                })
                .collect()
        })
    }
}

/// The items of a collection.
///
/// This is wrapper around a collection like a `Vec`. It implements [`Serialize`]
/// to serialize the items of the collection one after the other, but the length
/// is **not** serialized.
pub struct Items<'collection, Collection> {
    collection: &'collection Collection,
}

impl<'collection, C> Serialize for Items<'collection, C>
where
    C: SerializeItems,
{
    /// Serialize the items of the collection, but **not** its length.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        self.collection.serialize_items(serializer)
    }
}

impl<'collection, C> MultiPassSerialize for Items<'collection, C>
where
    C: MultiPassSerializeItems,
{
    /// Serialize the items of the collection, but **not** its length.
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        self.collection.serialize_items(serializer)
    }
}

/// Return the length of a collection as a specific (integer) type.
///
/// If the length of the collection can not be converted into the requested type
/// without losing precision, an error is returned.
pub fn len<T, S, C>(serializer: &mut S, collection: &C) -> Result<T, S::Error>
where
    S: Serializer,
    C: LenAs<T>,
{
    collection.len_as().ok_or_else(|| {
        serializer
            .error("the length of the collection is too large for its binary representation")
            .unwrap_err()
    })
}

/// Return the number of bytes an object occupies as serialized.
///
/// If the number of bytes cannot be converted into the requested type without
/// losing precision, an error is returned.
pub fn byte_count<T, Se, Sp>(serializer: &mut Se, span: &Sp) -> Result<T, Se::Error>
where
    T: TryFrom<u64>,
    Se: Serializer,
    Sp: Span,
{
    T::try_from(span.len()).map_err(|_| {
        serializer
            .error("the byte count of the collection is too large for its binary representation")
            .unwrap_err()
    })
}

/// Serialize the items in a collection, but not the length.
pub fn items<'collection, Collection>(collection: &'collection Collection) -> Items<'collection, Collection> {
    Items { collection }
}

/// Deserialize a collection given the number of its elements is given.
pub fn deserialize_items_by_len<Collection, Item, D, Len>(
    deserializer: &mut D,
    len: &Len,
) -> Result<Collection, D::Error>
where
    Collection: DeserializeByLen<Len, Item>,
    D: Deserializer,
    Len: Clone,
{
    Collection::deserialize_by_len(deserializer, len.clone())
}

/// Deserialize a collection given the number of bytes is given.
pub fn deserialize_items_by_byte_count<Collection, Item, D, Len>(
    deserializer: &mut D,
    byte_count: &Len,
) -> Result<Collection, D::Error>
where
    Collection: DeserializeByByteCount<Len, Item>,
    D: Deserializer,
    Len: Clone,
{
    Collection::deserialize_by_byte_count(deserializer, byte_count.clone())
}

#[cfg(test)]
mod tests {
    use crate::{collection::len, io::GrowingMemoryStream, stream_ser_de::StreamSerializer};

    #[test]
    fn len_() {
        let collection = vec![1, 2, 3];
        let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
        assert_eq!(len(&mut serializer, &collection), Ok(3));
    }
}
