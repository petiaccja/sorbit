use syn::{Index, Member, Type};

use crate::derive_struct::direct_field_attribute::DirectFieldAttribute;
use crate::derive_struct::field_utils::{
    lower_alignment, lower_deserialization_rounding, lower_offset, lower_serialization_rounding,
};
use crate::ir::dag::{Operation, Region, Value};
use crate::ir::ops::{ExecuteOp, MemberOp, SelfOp, YieldOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectField {
    pub name: Member,
    pub ty: Type,
    pub attribute: DirectFieldAttribute,
}

impl DirectField {
    pub fn parse(field: &syn::Field, index: usize) -> Result<Self, syn::Error> {
        let attribute = DirectFieldAttribute::parse(field.attrs.iter())?;
        let name = match &field.ident {
            Some(ident) => Member::Named(ident.clone()),
            None => Member::Unnamed(Index::from(index)),
        };
        let ty = field.ty.clone();
        Ok(Self { name, ty, attribute })
    }

    pub fn lower_se(&self, serializer: Value) -> Operation {
        ExecuteOp::new(Region::new(0, |_| {
            let mut ops = Vec::new();

            let self_ = SelfOp::new();
            let object = MemberOp::new(self_.output(), self.name.clone(), true);
            let object_val = object.output();
            ops.extend([self_.operation, object.operation].into_iter());

            lower_offset(serializer.clone(), self.attribute.offset, true, &mut ops);
            lower_alignment(serializer.clone(), self.attribute.align, true, &mut ops);
            let output = lower_serialization_rounding(serializer, object_val, self.attribute.round, &mut ops);
            ops.push(YieldOp::new(vec![output]).operation);

            ops
        }))
        .operation
    }

    pub fn lower_de(&self, deserializer: Value) -> Operation {
        ExecuteOp::new(Region::new(0, |_| {
            let mut ops = Vec::new();
            lower_offset(deserializer.clone(), self.attribute.offset, false, &mut ops);
            lower_alignment(deserializer.clone(), self.attribute.align, false, &mut ops);
            let output = lower_deserialization_rounding(deserializer, self.ty.clone(), self.attribute.round, &mut ops);
            ops.push(YieldOp::new(vec![output]).operation);
            ops
        }))
        .operation
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::pattern_match::assert_matches;

    use super::*;

    use quote::ToTokens;
    use syn::parse_quote;

    #[test]
    fn parse_trivial() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            foo: u8
        };
        let field = DirectField::parse(&input, 0)?;
        let expected =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(u8), attribute: DirectFieldAttribute::default() };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_layout() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(align=8)]
            foo: u8
        };
        let field = DirectField::parse(&input, 0)?;
        let expected = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: DirectFieldAttribute { offset: None, align: Some(8), round: None },
        };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_bit_field_decl() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, align=8)]
            foo: u8
        };
        assert!(DirectField::parse(&input, 0).is_err());
    }

    #[test]
    fn parse_bit_field_bits() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, bits(4..8))]
            foo: u8
        };
        assert!(DirectField::parse(&input, 0).is_err());
    }

    #[test]
    fn lower_se_default() {
        let input =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(i32), attribute: DirectFieldAttribute::default() };
        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_se(serializer));
        let pattern = "
        %out = execute || [
            %self = self
            %foo = member [foo, &] %self
            %res = serialize_object %serializer, %foo
            yield %res
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_se_layout() {
        let input = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(i32),
            attribute: DirectFieldAttribute { offset: Some(1), align: Some(2), round: Some(3) },
        };
        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_se(serializer.clone()));
        let pattern = "
        %out = execute || [
            %self = self
            %foo = member [foo, &] %self

            %offset = pad [1] %serializer
            %try_offset = try %offset

            %align = align [2] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| [
                %res_inner = serialize_object %s_inner, %foo
                %round = align [3] %s_inner
                %try_round = try %round
                yield %res_inner
            ]
            %res_try = try %res
            %res_1 = member [1, *] %res_try
            %res_ok = ok %res_1
            yield %res_ok
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_de_empty() {
        let input =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(i32), attribute: DirectFieldAttribute::default() };
        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_de(deserializer));
        let pattern = "
        %out = execute || [
            %res = deserialize_object [i32] %serializer
            yield %res
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_de_layout() {
        let input = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(i32),
            attribute: DirectFieldAttribute { offset: Some(1), align: Some(2), round: Some(3) },
        };
        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_de(deserializer.clone()));
        let pattern = "
        %out = execute || [
            %offset = pad [1] %deserializer
            %try_offset = try %offset

            %align = align [2] %deserializer
            %try_align = try %align

            %res = deserialize_composite %deserializer |%des_inner| [
                %res_inner = deserialize_object [i32] %des_inner
                %round = align [3] %des_inner
                %try_round = try %round
                yield %res_inner
            ]
            yield %res
        ]
        ";
        assert_matches!(op, pattern);
        println!("{}", input.lower_de(deserializer).to_token_stream())
    }
}
