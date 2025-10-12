use syn::Attribute;

use crate::shared::{parse_literal_int_meta, parse_meta_list_attr, sorbit_layout_path};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructAttribute {
    pub len: Option<u64>,
    pub round: Option<u64>,
}

impl StructAttribute {
    pub fn parse<'a>(attrs: impl Iterator<Item = &'a Attribute>) -> Result<Self, syn::Error> {
        let mut attributes = Self::default();
        let Self { len, round } = &mut attributes;

        for attr in attrs.filter(|attr| attr.path() == &sorbit_layout_path()) {
            parse_meta_list_attr(
                attr,
                &mut [
                    ("len", &mut |meta| parse_literal_int_meta(len, meta)),
                    ("round", &mut |meta| parse_literal_int_meta(round, meta)),
                ],
            )?;
        }

        Ok(attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::{Attribute, parse_quote};

    #[test]
    fn parse_none() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[derive(Clone)]
            #[sorbit::bit_field(A)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: None, round: None });
        Ok(())
    }

    #[test]
    fn parse_empty() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout()]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: None, round: None });
        Ok(())
    }

    #[test]
    fn parse_len() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(len=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: Some(4), round: None });
        Ok(())
    }

    #[test]
    fn parse_pad() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(round=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: None, round: Some(4) });
        Ok(())
    }

    #[test]
    fn parse_separate_parameters() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(len=8)]
            #[sorbit::layout(round=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: Some(8), round: Some(4) });
        Ok(())
    }

    #[test]
    fn parse_colocated_parameters() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(len=8, round=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: Some(8), round: Some(4) });
        Ok(())
    }

    #[test]
    fn parse_unrecognized() {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(foo)]
        };
        assert!(StructAttribute::parse(input.iter()).is_err());
    }
}
