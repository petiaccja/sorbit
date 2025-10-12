use std::collections::HashMap;

use proc_macro2::Span;
use syn::{Attribute, Type, spanned::Spanned};

use crate::shared::{
    parse_bit_field_name, parse_literal_int_meta, parse_meta_list_attr, parse_type_meta, placeholder_type,
    sorbit_bit_field_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitFieldAttribute {
    pub repr: Type,
    pub offset: Option<u64>,
    pub align: Option<u64>,
    pub round: Option<u64>,
}

impl BitFieldAttribute {
    pub fn parse_all<'a>(
        attrs: impl Iterator<Item = &'a Attribute>,
    ) -> Result<HashMap<String, BitFieldAttribute>, syn::Error> {
        let mut bit_fields = HashMap::<String, BitFieldAttribute>::new();
        let mut spans = HashMap::<String, Span>::new();

        for attr in attrs.filter(|attr| attr.path() == &sorbit_bit_field_path()) {
            let name = parse_bit_field_name(&attr.meta)?;
            let name_str = name.to_string();
            let attributes = bit_fields.entry(name_str.clone()).or_insert(BitFieldAttribute::default());
            let _ = spans.entry(name_str.clone()).or_insert(attr.span());

            let BitFieldAttribute { repr, offset, align, round } = attributes;
            parse_meta_list_attr(
                attr,
                &mut [
                    (name_str.as_str(), &mut |_meta| Ok(())), // Ignore the name parameter.
                    ("bits", &mut |_meta| Ok(())),            // Ignore the `bits` parameter.
                    ("repr", &mut |meta| parse_type_meta(repr, meta)),
                    ("offset", &mut |meta| parse_literal_int_meta(offset, meta)),
                    ("align", &mut |meta| parse_literal_int_meta(align, meta)),
                    ("round", &mut |meta| parse_literal_int_meta(round, meta)),
                ],
            )?;
        }

        for (name, bit_field) in bit_fields.iter() {
            if bit_field.repr == placeholder_type() {
                return Err(syn::Error::new(
                    spans.remove(name).expect("span must be inserted at the same time as the attribute"),
                    "bit field is missing repr parameter",
                ));
            }
        }

        Ok(bit_fields)
    }
}

impl Default for BitFieldAttribute {
    fn default() -> Self {
        Self { repr: placeholder_type(), offset: None, align: None, round: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::{Attribute, parse_quote};

    //--------------------------------------------------------------------------
    // Parsing.
    //--------------------------------------------------------------------------

    #[test]
    fn parse_none() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[derive(Debug)]
            #[sorbit::layout(round=4)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected = [];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_empty() {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field()]
        };
        assert!(BitFieldAttribute::parse_all(input.iter()).is_err());
    }

    #[test]
    fn parse_name() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8))]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected = [("A".to_string(), BitFieldAttribute { repr: parse_quote!(u8), ..Default::default() })];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_offset() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), offset=3)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected =
            [("A".to_string(), BitFieldAttribute { repr: parse_quote!(u8), offset: Some(3), ..Default::default() })];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_align() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), align=4)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected =
            [("A".to_string(), BitFieldAttribute { repr: parse_quote!(u8), align: Some(4), ..Default::default() })];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_round() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), round=5)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected =
            [("A".to_string(), BitFieldAttribute { repr: parse_quote!(u8), round: Some(5), ..Default::default() })];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_separate_parameters() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), offset=4)]
            #[sorbit::bit_field(A, align=5)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected = [(
            "A".to_string(),
            BitFieldAttribute { repr: parse_quote!(u8), offset: Some(4), align: Some(5), ..Default::default() },
        )];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_colocated_parameters() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), offset=4, align=5)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected = [(
            "A".to_string(),
            BitFieldAttribute { repr: parse_quote!(u8), offset: Some(4), align: Some(5), ..Default::default() },
        )];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_multiple_fields() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), offset=4)]
            #[sorbit::bit_field(B, repr(u8), offset=5)]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected = [
            ("A".to_string(), BitFieldAttribute { repr: parse_quote!(u8), offset: Some(4), ..Default::default() }),
            ("B".to_string(), BitFieldAttribute { repr: parse_quote!(u8), offset: Some(5), ..Default::default() }),
        ];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_ignored_parameters() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8), bits(4..6))]
        };
        let bit_fields = BitFieldAttribute::parse_all(input.iter())?;
        let expected = [("A".to_string(), BitFieldAttribute { repr: parse_quote!(u8), ..Default::default() })];
        assert_eq!(bit_fields, expected.into_iter().collect());
        Ok(())
    }

    #[test]
    fn parse_redefined_parameters() {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8))]
            #[sorbit::bit_field(A, repr(u8))]
        };
        assert!(BitFieldAttribute::parse_all(input.iter()).is_err());
    }

    #[test]
    fn parse_missing_repr() {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, offset=3)]
        };
        assert!(BitFieldAttribute::parse_all(input.iter()).is_err());
    }
}
