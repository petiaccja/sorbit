use std::ops::Range;
use syn::{Ident, Type, spanned::Spanned};

use super::utility::{
    parse_bit_field_name, parse_literal_int_meta, parse_literal_range_meta, parse_meta_list_attr, parse_type_meta,
    sorbit_bit_field_path, sorbit_layout_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Direct {
        ident: Option<Ident>,
        ty: Type,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
    Bit {
        ident: Option<Ident>,
        ty: Type,
        storage_id: Ident,
        storage_ty: Option<Type>,
        bits: Range<u8>,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
}

impl TryFrom<syn::Field> for Field {
    type Error = syn::Error;
    fn try_from(value: syn::Field) -> Result<Self, Self::Error> {
        let kind = Kind::of(&value)
            .ok_or(syn::Error::new(value.span(), "use only one of `layout` and `bit_field` attributes"))?;
        match kind {
            Kind::Direct => Self::parse_direct_field(value),
            Kind::Bit => Self::parse_bit_field(value),
        }
    }
}

impl Field {
    fn parse_direct_field(value: syn::Field) -> Result<Field, syn::Error> {
        let mut offset = None;
        let mut align = None;
        let mut round = None;

        let layout_attrs = value.attrs.iter().filter(|attr| attr.path() == &sorbit_layout_path());

        for attribute in layout_attrs {
            parse_meta_list_attr(
                attribute,
                &mut [
                    ("offset", &mut |meta| parse_literal_int_meta(&mut offset, meta)),
                    ("align", &mut |meta| parse_literal_int_meta(&mut align, meta)),
                    ("round", &mut |meta| parse_literal_int_meta(&mut round, meta)),
                ],
            )?;
        }

        Ok(Self::Direct { ident: value.ident, ty: value.ty, offset, align, round })
    }

    fn parse_bit_field(value: syn::Field) -> Result<Field, syn::Error> {
        let mut storage = None;
        let mut bits = None;
        let mut storage_ty = None;
        let mut offset = None;
        let mut align = None;
        let mut round = None;

        let bit_field_attrs = value.attrs.iter().filter(|attr| attr.path() == &sorbit_bit_field_path());

        for attribute in bit_field_attrs {
            let new_storage = parse_bit_field_name(&attribute.meta)?;
            if storage.is_some_and(|storage| storage != new_storage) {
                return Err(syn::Error::new(new_storage.span(), "previous attribute defines different storage"));
            }
            storage = Some(new_storage);
            parse_meta_list_attr(
                attribute,
                &mut [
                    (storage.as_ref().unwrap().to_string().as_str(), &mut |_meta| Ok(())), // Ignore the name parameter.
                    ("bits", &mut |meta| parse_literal_range_meta(&mut bits, meta)),
                    ("repr", &mut |meta| parse_type_meta(&mut storage_ty, meta)),
                    ("offset", &mut |meta| parse_literal_int_meta(&mut offset, meta)),
                    ("align", &mut |meta| parse_literal_int_meta(&mut align, meta)),
                    ("round", &mut |meta| parse_literal_int_meta(&mut round, meta)),
                ],
            )?;
        }

        let storage = storage.ok_or(syn::Error::new(value.span(), "missing storage identifier for bit field"))?;
        let bits = bits.ok_or(syn::Error::new(value.span(), "missing bit range for bit field"))?;

        Ok(Self::Bit { ident: value.ident, ty: value.ty, storage_id: storage, storage_ty, bits, offset, align, round })
    }
}

enum Kind {
    Direct,
    Bit,
}

impl Kind {
    pub fn of(field: &syn::Field) -> Option<Self> {
        let has_layout_attr = field.attrs.iter().find(|attr| attr.path() == &sorbit_layout_path()).is_some();
        let has_bit_field_attr = field.attrs.iter().find(|attr| attr.path() == &sorbit_bit_field_path()).is_some();
        match (has_layout_attr, has_bit_field_attr) {
            (true, true) => None,
            (true, false) => Some(Self::Direct),
            (false, true) => Some(Self::Bit),
            (false, false) => Some(Self::Direct),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn direct_default() {
        let input: syn::Field = parse_quote! {
            field: u8
        };
        let actual = Field::try_from(input);
        let expected =
            Field::Direct { ident: parse_quote!(field), ty: parse_quote!(u8), offset: None, align: None, round: None };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_foreign_attribute() {
        let input: syn::Field = parse_quote! {
            #[derive(Debug)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected =
            Field::Direct { ident: parse_quote!(field), ty: parse_quote!(u8), offset: None, align: None, round: None };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_with_layout_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(offset=1, align=2, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_with_layout_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(offset=1)]
            #[sorbit_layout(align=2)]
            #[sorbit_layout(round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    #[should_panic]
    fn direct_with_layout_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(align=2)]
            #[sorbit_layout(align=3)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn direct_invalid_meta_key() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(align=2, invalid_key=4)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn direct_invalid_meta_value() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(align=invalid_value)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    fn bit_default_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: None,
            align: None,
            round: None,
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_default_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field)]
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: None,
            align: None,
            round: None,
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_foreign_attribute() {
        let input: syn::Field = parse_quote! {
            #[derive(Debug)]
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: None,
            align: None,
            round: None,
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_with_layout_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3), offset=1, align=2, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_with_layout_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            #[sorbit_bit_field(_bit_field, offset=1)]
            #[sorbit_bit_field(_bit_field, align=2)]
            #[sorbit_bit_field(_bit_field, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    #[should_panic]
    fn bit_bits_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_name_missing() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(bits(1..3))]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_bits_missing() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_with_layout_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3), offset=1)]
            #[sorbit_bit_field(_bit_field, offset=2)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_invalid_meta_key() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3), invalid_key=1)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_invalid_meta_value() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(_bit_field, bits(1..3), offset=invalid_value)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn ambiguous_kind() {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout()]
            #[sorbit_bit_field(_bit_field, bits(1..3))]
            field: u8
        };
        Field::try_from(input).unwrap();
    }
}
