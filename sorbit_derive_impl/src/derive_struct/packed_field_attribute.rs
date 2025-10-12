use std::ops::Range;

use proc_macro2::Span;
use syn::{Attribute, Ident, spanned::Spanned};

use crate::shared::{
    parse_bit_field_name, parse_literal_range_meta, parse_meta_list_attr, sorbit_bit_field_path, sorbit_layout_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedFieldAttribute {
    pub storage: Ident,
    pub bits: Range<u8>,
}

impl PackedFieldAttribute {
    pub fn parse<'a>(attrs: impl Iterator<Item = &'a Attribute>) -> Result<Self, syn::Error> {
        let mut storage: Option<Ident> = None;
        let mut bits: Option<Range<u8>> = None;

        for attr in attrs {
            if attr.path() == &sorbit_bit_field_path() {
                let name = parse_bit_field_name(&attr.meta)?;
                let name_str = name.to_string();

                parse_meta_list_attr(
                    attr,
                    &mut [
                        (name_str.as_str(), &mut |_meta| Ok(())), // Ignore the name parameter.
                        ("bits", &mut |meta| parse_literal_range_meta(&mut bits, meta)),
                        ("repr", &mut |_meta| Ok(())),   // Ignore storage definition.
                        ("offset", &mut |_meta| Ok(())), // Ignore storage definition.
                        ("align", &mut |_meta| Ok(())),  // Ignore storage definition.
                        ("round", &mut |_meta| Ok(())),  // Ignore storage definition.
                    ],
                )?;

                // The first time we see the `bits` parameter, we save the `name` as the containing bit field.
                // The second time we see the `bits` parameter the above parsing will fail.
                if bits.is_some() {
                    storage = Some(name);
                }
            } else if attr.path() == &sorbit_layout_path() {
                return Err(syn::Error::new(
                    attr.span(),
                    "sorbit::layour attributes are not allowed on bit field members",
                ));
            }
        }

        match (storage, bits) {
            (Some(storage), Some(bits)) => Ok(Self { storage, bits }),
            _ => Err(syn::Error::new(Span::call_site(), "the `bits` field is mandatory")),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn parse_none() {
        let input = [];
        assert!(PackedFieldAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_ignore_foreign() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[derive(Clone)]
            #[sorbit_bit_field(A, bits(0..1))]
        );
        let result = PackedFieldAttribute::parse(input.iter())?;
        let expected = PackedFieldAttribute { storage: parse_quote!(A), bits: 0..1 };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_empty() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field()]
        );
        assert!(PackedFieldAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_layout() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout()]
        );
        assert!(PackedFieldAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_ignore_storage() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(0..1), repr(u8), offset=3, align=4, round=5)]
        );
        let result = PackedFieldAttribute::parse(input.iter())?;
        let expected = PackedFieldAttribute { storage: parse_quote!(A), bits: 0..1 };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_half_open_range() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(0..3))]
        );
        let result = PackedFieldAttribute::parse(input.iter())?;
        let expected = PackedFieldAttribute { storage: parse_quote!(A), bits: 0..3 };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_closed_range() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(0..=2))]
        );
        let result = PackedFieldAttribute::parse(input.iter())?;
        let expected = PackedFieldAttribute { storage: parse_quote!(A), bits: 0..3 };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_single_bit() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(1))]
        );
        let result = PackedFieldAttribute::parse(input.iter())?;
        let expected = PackedFieldAttribute { storage: parse_quote!(A), bits: 1..2 };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_unbounded_left() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(0..))]
        );
        assert!(PackedFieldAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_unbounded_right() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(..2))]
        );
        assert!(PackedFieldAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_unbounded_both() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A, bits(..))]
        );
        assert!(PackedFieldAttribute::parse(input.iter()).is_err());
    }
}
