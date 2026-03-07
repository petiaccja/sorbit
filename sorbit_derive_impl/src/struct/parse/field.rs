use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};
use syn::{Expr, Ident, Path, Type, spanned::Spanned};

use crate::attribute::{
    BitNumbering, ByteOrder, Transform, as_bit_numbering, as_byte_order, as_ident, as_literal_int,
    as_literal_int_range, as_transform, as_type, parse_nvp_attribute_group, path,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldLayoutProperties {
    pub byte_order: Option<ByteOrder>,
    pub offset: Option<u64>,
    pub align: Option<u64>,
    pub round: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BitFieldStorageProperties {
    pub storage_ty: Option<Type>,
    pub bit_numbering: Option<BitNumbering>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Direct {
        ident: Option<Ident>,
        ty: Type,
        transform: Transform,
        layout_properties: FieldLayoutProperties,
    },
    Bit {
        ident: Option<Ident>,
        ty: Type,
        transform: Transform,
        bits: Range<u8>,
        storage_ident: Ident,
        storage_properties: BitFieldStorageProperties,
        layout_properties: FieldLayoutProperties,
    },
}

impl TryFrom<syn::Field> for Field {
    type Error = syn::Error;
    fn try_from(value: syn::Field) -> Result<Self, Self::Error> {
        let sorbit_attrs = value.attrs.iter().filter(|attr| attr.path() == &path::sorbit_attribute());
        let parameters = parse_nvp_attribute_group(sorbit_attrs)?;
        let is_bit_field = parameters.contains_key(&path::storage_id()) || parameters.contains_key(&path::bit_range());

        if !is_bit_field {
            Self::parse_direct_field(value.ident, value.ty, parameters)
        } else {
            Self::parse_bit_field(value.ident, value.ty, parameters)
        }
    }
}

impl Field {
    fn parse_direct_field(
        ident: Option<Ident>,
        ty: Type,
        parameters: HashMap<Path, Expr>,
    ) -> Result<Field, syn::Error> {
        let accepted_parameters = FieldLayoutProperties::accepted_parameters();
        check_invalid_parameters(&parameters, accepted_parameters.iter())?;

        let transform = parameters.get(&path::value()).map(as_transform).transpose()?.unwrap_or_default();
        let layout_properties = FieldLayoutProperties::from_parameters(&parameters)?;
        Ok(Self::Direct { ident, ty, transform, layout_properties })
    }

    fn parse_bit_field(ident: Option<Ident>, ty: Type, parameters: HashMap<Path, Expr>) -> Result<Field, syn::Error> {
        let accepted_parameters = [
            &[path::bit_range(), path::storage_id()] as &[Path],
            &BitFieldStorageProperties::accepted_parameters() as &[Path],
            &FieldLayoutProperties::accepted_parameters() as &[Path],
        ];
        check_invalid_parameters(&parameters, accepted_parameters.into_iter().flatten())?;

        let transform = parameters.get(&path::value()).map(as_transform).transpose()?.unwrap_or_default();
        let bits = parameters
            .get(&path::bit_range())
            .map(|expr| {
                as_literal_int_range(expr)
                    .or(as_literal_int(expr).map(|bit| bit..(bit + 1)))
                    .map_err(|err| syn::Error::new(err.span(), "expected either a literal range or an integer literal"))
            })
            .ok_or(syn::Error::new(
                ident.span(),
                "this bit field is missing the bit range, add `bits=<S>..<E>` or `bits=<B>` to the attribute",
            ))??;
        let storage_ident = parameters.get(&path::storage_id()).map(as_ident).ok_or(syn::Error::new(
            ident.span(),
            "this bit field is missing the storage identifier, add `bit_field=<IDENTIFIER>` to the attribute",
        ))??;
        let storage_properties = BitFieldStorageProperties::from_parameters(&parameters)?;
        let layout_properties = FieldLayoutProperties::from_parameters(&parameters)?;

        Ok(Self::Bit { ident, ty, transform, bits, storage_ident, storage_properties, layout_properties })
    }
}

impl FieldLayoutProperties {
    pub fn from_parameters(parameters: &HashMap<Path, Expr>) -> Result<Self, syn::Error> {
        let byte_order = parameters.get(&path::byte_order()).map(as_byte_order).transpose()?;
        let offset = parameters.get(&path::offset()).map(as_literal_int).transpose()?;
        let align = parameters.get(&path::align()).map(as_literal_int).transpose()?;
        let round = parameters.get(&path::round()).map(as_literal_int).transpose()?;
        Ok(Self { byte_order, offset, align, round })
    }

    pub fn accepted_parameters() -> [Path; 4] {
        [
            path::byte_order(),
            path::offset(),
            path::align(),
            path::round(),
        ]
    }
}

impl BitFieldStorageProperties {
    pub fn from_parameters(parameters: &HashMap<Path, Expr>) -> Result<Self, syn::Error> {
        let storage_ty = parameters.get(&path::storage_ty()).map(as_type).transpose()?;
        let bit_numbering = parameters.get(&path::bit_numbering()).map(as_bit_numbering).transpose()?;
        Ok(Self { storage_ty, bit_numbering })
    }

    pub fn accepted_parameters() -> [Path; 2] {
        [path::storage_ty(), path::bit_numbering()]
    }
}

fn check_invalid_parameters<'a>(
    parameters: &HashMap<Path, Expr>,
    accepted_parameters: impl Iterator<Item = &'a Path>,
) -> Result<(), syn::Error> {
    let accepted_parameters: HashSet<_> = accepted_parameters.cloned().collect();
    for (parameter, _) in parameters {
        if !accepted_parameters.contains(parameter) {
            return Err(syn::Error::new(parameter.span(), "parameter is not accepted here"));
        }
    }
    Ok(())
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
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            layout_properties: Default::default(),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_foreign_attribute() {
        let input: syn::Field = parse_quote! {
            #[derive(Debug)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            layout_properties: Default::default(),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_with_layout_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit(offset=1, align=2, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties {
                byte_order: None,
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_with_layout_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit(offset=1)]
            #[sorbit(align=2)]
            #[sorbit(round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties {
                byte_order: None,
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    #[should_panic]
    fn direct_with_layout_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit(align=2)]
            #[sorbit(align=3)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn direct_invalid_meta_key() {
        let input: syn::Field = parse_quote! {
            #[sorbit(align=2, invalid_key=4)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn direct_invalid_meta_value() {
        let input: syn::Field = parse_quote! {
            #[sorbit(align=invalid_value)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    fn bit_default_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            bits: 1..3,
            storage_ident: parse_quote!(_bit_field),
            storage_properties: Default::default(),
            layout_properties: Default::default(),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_default_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field)]
            #[sorbit(bits=1..3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            bits: 1..3,
            storage_ident: parse_quote!(_bit_field),
            storage_properties: Default::default(),
            layout_properties: Default::default(),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_foreign_attribute() {
        let input: syn::Field = parse_quote! {
            #[derive(Debug)]
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            bits: 1..3,
            storage_ident: parse_quote!(_bit_field),
            storage_properties: Default::default(),
            layout_properties: Default::default(),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_with_layout_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, offset=1, align=2, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            bits: 1..3,
            storage_ident: parse_quote!(_bit_field),
            storage_properties: Default::default(),
            layout_properties: FieldLayoutProperties {
                byte_order: None,
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_with_layout_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            #[sorbit(offset=1)]
            #[sorbit(align=2)]
            #[sorbit(round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            transform: Transform::None,
            bits: 1..3,
            storage_ident: parse_quote!(_bit_field),
            storage_properties: Default::default(),
            layout_properties: FieldLayoutProperties {
                byte_order: None,
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    #[should_panic]
    fn bit_bits_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            #[sorbit(bits=1..4)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_name_missing() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bits=1..3)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_bits_missing() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_with_layout_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, offset=1)]
            #[sorbit(offset=2)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_invalid_meta_key() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, invalid_key=1)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_invalid_meta_value() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, offset=invalid_value)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }
}
