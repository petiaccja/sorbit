use crate::derive_struct::{direct_field::DirectField, packed_field::PackedField};
use crate::parse_utils::sorbit_bit_field_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceField {
    Direct(DirectField),
    Packed(PackedField),
}

impl SourceField {
    pub fn parse(field: &syn::Field, index: usize) -> Result<Self, syn::Error> {
        let is_bit_field = field.attrs.iter().find(|attr| attr.path() == &sorbit_bit_field_path()).is_some();
        if is_bit_field {
            Ok(SourceField::Packed(PackedField::parse(field, index)?))
        } else {
            Ok(SourceField::Direct(DirectField::parse(field, index)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::derive_struct::direct_field_attribute::DirectFieldAttribute;
    use crate::derive_struct::packed_field_attribute::PackedFieldAttribute;

    use super::*;

    use syn::parse_quote;

    #[test]
    fn parse_declared_field_direct_field_empty() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            foo: u8
        };
        let field = SourceField::parse(&input, 0)?;
        let expected = SourceField::Direct(DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: DirectFieldAttribute { offset: None, align: None, round: None },
        });
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_declared_field_direct_field_with_params() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(offset=10)]
            foo: u8
        };
        let field = SourceField::parse(&input, 0)?;
        let expected = SourceField::Direct(DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: DirectFieldAttribute { offset: Some(10), align: None, round: None },
        });
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_declared_field_bit_field() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, bits(3..4))]
            foo: u8
        };
        let field = SourceField::parse(&input, 0)?;
        let expected = SourceField::Packed(PackedField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 3..4 },
        });
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_declared_field_ambiguous() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(offset=0)]
            #[sorbit_bit_field(A, bits(3..4))]
            foo: u8
        };
        assert!(SourceField::parse(&input, 0).is_err());
    }
}
