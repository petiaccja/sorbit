//! Utilities for serializing collections, like `Vec`.

use crate::ser_de::{Deserialize, Deserializer, MultiPassSerialize, RevisableSerializer, Serialize, Serializer, Span};

/// Return the length of a collection as a specific (integer) type.
///
/// If the length of the collection can not be converted into the requested type
/// without losing precision, an error is returned.
pub fn len<'collection, Len, SerializerTy, Collection>(
    serializer: &mut SerializerTy,
    collection: &'collection Collection,
) -> Result<Len, SerializerTy::Error>
where
    Len: TryFrom<usize>,
    SerializerTy: Serializer,
    &'collection Collection: IntoIterator<IntoIter: ExactSizeIterator>,
{
    if let Ok(len) = Len::try_from(collection.into_iter().len()) {
        Ok(len)
    } else {
        serializer.error("the length of the collection is too large for its binary representation")
    }
}

/// Return the number of bytes an object occupies as serialized.
///
/// If the number of bytes cannot be converted into the requested type without
/// losing precision, an error is returned.
pub fn byte_count<ByteCount, SerializerTy, SpanTy>(
    serializer: &mut SerializerTy,
    span: &SpanTy,
) -> Result<ByteCount, SerializerTy::Error>
where
    ByteCount: TryFrom<u64>,
    SerializerTy: Serializer,
    SpanTy: Span,
{
    if let Ok(len) = ByteCount::try_from(span.len()) {
        Ok(len)
    } else {
        serializer.error("the byte count of the collection is too large for its binary representation")
    }
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
    D: Deserializer,
    Item: Deserialize,
    Collection: FromIterator<Item>,
    usize: TryFrom<Len>,
    Len: Clone,
{
    let Ok(len) = usize::try_from(len.clone()) else {
        return deserializer.error("the length of the collection can not be converted into a `usize`");
    };
    (0..len).into_iter().map(|_| Item::deserialize(deserializer)).collect()
}

/// Deserialize a collection given the number of bytes is given.
pub fn deserialize_items_by_byte_count<Collection, Item, D, Len>(
    deserializer: &mut D,
    byte_count: &Len,
) -> Result<Collection, D::Error>
where
    D: Deserializer,
    Item: Deserialize,
    Collection: FromIterator<Item>,
    usize: TryFrom<Len>,
    Len: Clone,
{
    let Ok(byte_count) = usize::try_from(byte_count.clone()) else {
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

/// The items of a collection.
///
/// This is wrapper around a collection like a `Vec`. It implements [`Serialize`]
/// to serialize the items of the collection one after the other, but the length
/// is **not** serialized.
pub struct Items<'collection, Collection> {
    collection: &'collection Collection,
}

impl<'collection, Collection> Serialize for Items<'collection, Collection>
where
    for<'a> &'a Collection: IntoIterator<Item: Serialize>,
{
    /// Serialize the items of the collection, but **not** its length.
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer
            .serialize_composite(|serializer| {
                for item in self.collection {
                    item.serialize(serializer)?;
                }
                serializer.success()
            })
            .map(|(composite_span, _)| composite_span)
    }
}

impl<'collection, Collection> MultiPassSerialize for Items<'collection, Collection>
where
    for<'a> &'a Collection: IntoIterator<Item: MultiPassSerialize>,
{
    /// Serialize the items of the collection, but **not** its length.
    fn serialize<S: RevisableSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        serializer
            .serialize_composite(|serializer| {
                for item in self.collection {
                    item.serialize(serializer)?;
                }
                serializer.success()
            })
            .map(|(composite_span, _)| composite_span)
    }
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
