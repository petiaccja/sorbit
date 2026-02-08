use syn::{Attribute, spanned::Spanned};

use crate::parse_utils::{parse_literal_int_meta, parse_meta_list_attr, sorbit_bit_field_path, sorbit_layout_path};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectFieldAttribute {
    pub offset: Option<u64>,
    pub align: Option<u64>,
    pub round: Option<u64>,
}

impl DirectFieldAttribute {
    pub fn parse<'a>(attrs: impl Iterator<Item = &'a Attribute>) -> Result<Self, syn::Error> {
        let mut attributes = Self::default();
        let Self { offset, align, round } = &mut attributes;

        for attr in attrs {
            if attr.path() == &sorbit_layout_path() {
                parse_meta_list_attr(
                    attr,
                    &mut [
                        ("offset", &mut |meta| parse_literal_int_meta(offset, meta)),
                        ("align", &mut |meta| parse_literal_int_meta(align, meta)),
                        ("round", &mut |meta| parse_literal_int_meta(round, meta)),
                    ],
                )?;
            } else if attr.path() == &sorbit_bit_field_path() {
                return Err(syn::Error::new(
                    attr.span(),
                    "sorbit_bit_field attributes are not allowed on direct fields",
                ));
            }
        }

        Ok(attributes)
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn parse_none() -> Result<(), syn::Error> {
        let input = [];
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute::default();
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_ignore() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[derive(Clone)]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute::default();
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_empty() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout()]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute::default();
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_bit_field() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_bit_field(A)]
        );
        assert!(DirectFieldAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_offset() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout(offset=4)]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute { offset: Some(4), ..Default::default() };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_align() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout(align=4)]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute { align: Some(4), ..Default::default() };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_round() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout(round=4)]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute { round: Some(4), ..Default::default() };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_multiple_together() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout(round=4, align=6)]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute { round: Some(4), align: Some(6), ..Default::default() };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_multiple_separate() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout(round=4)]
            #[sorbit_layout(align=6)]
        );
        let result = DirectFieldAttribute::parse(input.iter())?;
        let expected = DirectFieldAttribute { round: Some(4), align: Some(6), ..Default::default() };
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn parse_invalid() {
        let input: Vec<Attribute> = parse_quote!(
            #[sorbit_layout(round=4, invalid_param=9)]
        );
        assert!(DirectFieldAttribute::parse(input.iter()).is_err());
    }
}
