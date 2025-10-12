use syn::{Index, Member, Type};

use crate::derive_struct::packed_field_attribute::PackedFieldAttribute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedField {
    pub name: Member,
    pub ty: Type,
    pub attribute: PackedFieldAttribute,
}

impl PackedField {
    pub fn parse(field: &syn::Field, index: usize) -> Result<Self, syn::Error> {
        let attribute = PackedFieldAttribute::parse(field.attrs.iter())?;
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

    use crate::derive_struct::packed_field_attribute::PackedFieldAttribute;

    #[test]
    fn parse_trivial() {
        let input: syn::Field = parse_quote! {
            foo: u8
        };
        assert!(PackedField::parse(&input, 0).is_err());
    }

    #[test]
    fn parse_storage_only() {
        let input: syn::Field = parse_quote! {
            #[sorbit::bit_field(A)]
            foo: u8
        };
        assert!(PackedField::parse(&input, 0).is_err());
    }

    #[test]
    fn parse_bits_only() {
        let input: syn::Field = parse_quote! {
            #[sorbit::bit_field(bits(3..4))]
            foo: u8
        };
        assert!(PackedField::parse(&input, 0).is_err());
    }

    #[test]
    fn parse_storage_and_bits() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit::bit_field(A, bits(3..4))]
            foo: u8
        };
        let field = PackedField::parse(&input, 0)?;
        let expected = PackedField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 3..4 },
        };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_storage_and_bits_separate() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit::bit_field(A, offset=4)]
            #[sorbit::bit_field(A, bits(3..4))]
            foo: u8
        };
        let field = PackedField::parse(&input, 0)?;
        let expected = PackedField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 3..4 },
        };
        assert_eq!(field, expected);
        Ok(())
    }
}
