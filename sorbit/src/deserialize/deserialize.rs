use crate::deserialize::Deserializer;

pub trait Deserialize
where
    Self: Sized,
{
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error>;
}
