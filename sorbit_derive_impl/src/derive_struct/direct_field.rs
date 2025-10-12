use syn::{Index, Member, Type};

use crate::derive_struct::direct_field_attribute::DirectFieldAttribute;

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
}
