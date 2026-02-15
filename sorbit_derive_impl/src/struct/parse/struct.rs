use syn::{DeriveInput, Generics, Ident, spanned::Spanned};

use super::field::Field;
use super::utility::{parse_literal_int_meta, parse_meta_list_attr, sorbit_layout_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    pub ident: Ident,
    pub generics: Generics,
    pub len: Option<u64>,
    pub round: Option<u64>,
    pub fields: Vec<Field>,
}

impl TryFrom<DeriveInput> for Struct {
    type Error = syn::Error;
    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        match value.data {
            syn::Data::Struct(data_struct) => {
                let mut len = None;
                let mut round = None;

                let layout_attrs = value.attrs.iter().filter(|attr| attr.path() == &sorbit_layout_path());
                for attribute in layout_attrs {
                    parse_meta_list_attr(
                        attribute,
                        &mut [
                            ("len", &mut |meta| parse_literal_int_meta(&mut len, meta)),
                            ("round", &mut |meta| parse_literal_int_meta(&mut round, meta)),
                        ],
                    )?;
                }

                let fields = data_struct
                    .fields
                    .into_iter()
                    .map(|field| Field::try_from(field))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self { ident: value.ident, generics: value.generics, len, round, fields })
            }
            syn::Data::Enum(_) => Err(syn::Error::new(value.span(), "expected a struct, got an enum")),
            syn::Data::Union(_) => Err(syn::Error::new(value.span(), "expected a struct, got a union")),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn default() {
        let input: DeriveInput = parse_quote!(
            struct Struct {}
        );
        let actual = Struct::try_from(input).unwrap();
        let expected = Struct {
            ident: parse_quote!(Struct),
            generics: Generics::default(),
            len: None,
            round: None,
            fields: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn with_layout_merged() {
        let input: DeriveInput = parse_quote!(
            #[sorbit_layout(len = 1, round = 2)]
            struct Struct {}
        );
        let actual = Struct::try_from(input).unwrap();
        let expected = Struct {
            ident: parse_quote!(Struct),
            generics: Generics::default(),
            len: Some(1),
            round: Some(2),
            fields: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn with_layout_split() {
        let input: DeriveInput = parse_quote!(
            #[sorbit_layout(len = 1)]
            #[sorbit_layout(round = 2)]
            struct Struct {}
        );
        let actual = Struct::try_from(input).unwrap();
        let expected = Struct {
            ident: parse_quote!(Struct),
            generics: Generics::default(),
            len: Some(1),
            round: Some(2),
            fields: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn foreign_attribute() {
        let input: DeriveInput = parse_quote!(
            #[derive(Debug)]
            struct Struct {}
        );
        let actual = Struct::try_from(input).unwrap();
        let expected = Struct {
            ident: parse_quote!(Struct),
            generics: Generics::default(),
            len: None,
            round: None,
            fields: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn with_fields() {
        let input: DeriveInput = parse_quote!(
            struct Struct {
                field: u8,
            }
        );
        let actual = Struct::try_from(input).unwrap();
        let expected = Struct {
            ident: parse_quote!(Struct),
            generics: Generics::default(),
            len: None,
            round: None,
            fields: vec![Field::Direct {
                ident: parse_quote!(field),
                ty: parse_quote!(u8),
                offset: None,
                align: None,
                round: None,
            }],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn invalid_key() {
        let input: DeriveInput = parse_quote!(
            #[sorbit_layout(invalid_key = 1)]
            struct Struct {}
        );
        Struct::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_value() {
        let input: DeriveInput = parse_quote!(
            #[sorbit_layout(len=invalid_value)]
            struct Struct {}
        );
        Struct::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn unexpected_derive_input() {
        let input: DeriveInput = parse_quote!(
            enum Enum {}
        );
        Struct::try_from(input).unwrap();
    }
}
