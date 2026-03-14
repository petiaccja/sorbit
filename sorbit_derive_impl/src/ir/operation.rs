use quote::ToTokens;
use std::ops::Index;

use crate::ir::{Region, Value};

pub trait Operation: ToTokens {
    fn name(&self) -> &str;
    fn is_terminator(&self) -> bool {
        false
    }
    fn inputs(&self) -> Vec<Value>;
    fn outputs(&self) -> Vec<Value>;
    fn regions(&self) -> Vec<&Region>;
    fn attributes(&self) -> Vec<String>;
    fn to_string(&self, alternate: bool) -> String {
        let outputs = self.outputs().iter().map(|output| format!("{output}")).collect::<Vec<_>>().join(", ");
        let inputs = self.inputs().iter().map(|input| format!("{input}")).collect::<Vec<_>>().join(", ");
        let attributes = self.attributes().join(", ");
        let regions = self
            .regions()
            .iter()
            .map(|region| if alternate { format!("{region:#}") } else { format!("{region}") })
            .collect::<Vec<_>>()
            .join(" ");
        let mut s = String::new();
        if !outputs.is_empty() {
            s.push_str(&format!("{outputs} = "));
        };
        s.push_str(self.name());
        if !attributes.is_empty() {
            s.push_str(&format!(" [{attributes}]"));
        };
        if !inputs.is_empty() {
            s.push_str(&format!(" {inputs}"));
        };
        if !regions.is_empty() {
            s.push_str(&format!(" {regions}"));
        }
        s
    }
}

macro_rules! value {
    ($_:ident) => {
        crate::ir::Value
    };
}

pub trait IntoValueTuple<Tuple> {
    fn into_value_tuple(&self) -> Tuple;
}

macro_rules! impl_into_value_tuple {
    ($tuple:ty, $($indices:expr),*) => {
        impl<C> IntoValueTuple<$tuple> for C
            where Self: Index<usize, Output = Value>
        {
            fn into_value_tuple(&self) -> $tuple {
                ($((self)[$indices]),*)
            }
        }
    };
}

impl_into_value_tuple!((),);
impl_into_value_tuple!(Value, 0);
impl_into_value_tuple!((Value, Value), 0, 1);
impl_into_value_tuple!((Value, Value, Value), 0, 1, 2);
impl_into_value_tuple!((Value, Value, Value, Value), 0, 1, 2, 3);
impl_into_value_tuple!((Value, Value, Value, Value, Value), 0, 1, 2, 3, 4);
impl_into_value_tuple!((Value, Value, Value, Value, Value, Value), 0, 1, 2, 3, 4, 5);
impl_into_value_tuple!((Value, Value, Value, Value, Value, Value, Value), 0, 1, 2, 3, 4, 5, 6);
impl_into_value_tuple!((Value, Value, Value, Value, Value, Value, Value, Value), 0, 1, 2, 3, 4, 5, 6, 7);

macro_rules! op {
    (
        name: $name:expr,
        builder: $builder:ident,
        op: $op:ident,
        inputs: {$($inputs:ident),*},
        outputs: {$($outputs:ident),*},
        attributes: {$($attributes:ident: $attribute_tys:ty),*},
        regions: {$($regions:ident),*},
        terminator: $terminator:expr$(,)?
    ) => {
        #[allow(unused_parens)]
        pub fn $builder(
            region: &mut crate::ir::Region,
            $($inputs: crate::ir::Value,)*
            $($attributes: $attribute_tys,)*
            $($regions: crate::ir::Region,)*
        ) -> ($(crate::ir::operation::value!($outputs)),*) {
            #[allow(unused)]
            let op = $op {
                $($inputs,)*
                $($outputs: crate::ir::Value::new(),)*
                $($attributes,)*
                $($regions,)*
            };
            let result = region.push(op);
            crate::ir::operation::IntoValueTuple::into_value_tuple(&result)
        }

        pub struct $op {
            $($inputs: crate::ir::Value,)*
            $($outputs: crate::ir::Value,)*
            $($attributes: $attribute_tys,)*
            $($regions: crate::ir::Region,)*
        }

        impl crate::ir::Operation for $op {
            fn name(&self) -> &str {
                $name
            }

            fn is_terminator(&self) -> bool {
                $terminator
            }

            fn inputs(&self) -> Vec<crate::ir::Value> {
                vec![$((self).$inputs),*]
            }

            fn outputs(&self) -> Vec<crate::ir::Value> {
                vec![$((self).$outputs),*]
            }

            fn regions(&self) -> Vec<&crate::ir::Region> {
                vec![$(&(self).$regions),*]
            }

            fn attributes(&self) -> Vec<String> {
                vec![$((crate::ir::Attribute::display(&(self).$attributes))),*]
            }
        }
    }
}

pub(crate) use op;
pub(crate) use value;
