use syn::{Index, Member, Type, parse_quote};

use crate::derive_struct::direct_field_attribute::DirectFieldAttribute;
use crate::derive_struct::field_utils::member_to_ident;
use crate::{ir_de, ir_se};

use super::field_utils::{lower_de_with_layout, lower_se_with_layout, member_to_string};

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

    pub fn lower_se(&self) -> ir_se::Expr {
        let member = &self.name;
        let name = member_to_string(&self.name);
        lower_se_with_layout(
            &parse_quote!(&self.#member),
            Some(&name),
            self.attribute.offset,
            self.attribute.align,
            self.attribute.round,
        )
    }

    pub fn lower_de(&self) -> Vec<ir_de::Let> {
        let name = member_to_string(&self.name);
        lower_de_with_layout(
            &member_to_ident(&self.name),
            &self.ty,
            Some(&name),
            self.attribute.offset,
            self.attribute.align,
            self.attribute.round,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn lower_se_all() {
        let input =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(i32), attribute: DirectFieldAttribute::default() };
        let expected = ir_se::enclose(ir_se::serialize_object(parse_quote!(&self.foo)), "foo".into());
        let actual = input.lower_se();
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_de_all() {
        let input =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(i32), attribute: DirectFieldAttribute::default() };
        let expected = [ir_de::r#let(
            parse_quote!(foo),
            ir_de::r#try(ir_de::enclose(ir_de::deserialize_object(parse_quote!(i32)), "foo".into())),
        )];
        let actual = input.lower_de();
        assert_eq!(&actual, &expected);
    }
}
