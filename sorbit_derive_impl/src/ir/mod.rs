mod attribute;
pub mod operation;
#[cfg(test)]
pub mod pattern_match;
mod region;
mod value;

pub use attribute::Attribute;
pub use operation::Operation;
pub use region::Region;
pub use value::Value;

pub trait ToSerializeOp {
    type Args;
    fn to_serialize_op(&self, region: &mut Region, args: Self::Args) -> Vec<Value>;
}

pub trait ToDeserializeOp {
    type Args;
    fn to_deserialize_op(&self, region: &mut Region, args: Self::Args) -> Vec<Value>;
}

pub(crate) use operation::op;
