use std::{collections::HashMap, ops::Range};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, DeriveInput, Generics, Ident, LitInt, Meta, Path, Type, meta::ParseNestedMeta, parse::ParseStream,
    parse_quote, punctuated::Punctuated, spanned::Spanned, token::Comma,
};

use crate::shared::{
    parse_literal_int_meta, parse_meta_list_attr, parse_type_meta, placeholder_type, sorbit_bit_field_path,
    sorbit_layout_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    name: Ident,
    generics: Generics,
    attributes: StructAttribute,
    fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructAttribute {
    len: Option<u64>,
    round: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlainFieldAttribute {
    offset: Option<u64>,
    align: Option<u64>,
    round: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitFieldAttribute {
    repr: Type,
    offset: Option<u64>,
    align: Option<u64>,
    round: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    Regular(PlainField),
    Bit(BitField),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainField {
    name: Ident,
    ty: Type,
    attribute: PlainFieldAttribute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitField {
    name: Ident,
    attribute: BitFieldAttribute,
    members: Vec<BitFieldMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitFieldMember {
    name: Ident,
    ty: Type,
    bits: Range<u8>,
}

impl Struct {
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        let syn::Data::Struct(data) = &input.data else {
            return Err(syn::Error::new(input.span(), "expected a struct"));
        };
        let name = input.ident.clone();
        let generics = input.generics.clone();
        let attributes = StructAttribute::parse(input.attrs.iter())?;

        Ok(Self { name, generics, attributes, fields: vec![] })
    }

    pub fn derive_serialize(&self) -> TokenStream {
        TokenStream::new()
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        TokenStream::new()
    }
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

impl PlainFieldAttribute {
    pub fn parse<'a>(attrs: impl Iterator<Item = &'a Attribute>) -> Result<Self, syn::Error> {
        let mut attributes = Self::default();
        let Self { offset, align, round } = &mut attributes;

        for attr in attrs.filter(|attr| attr.path() == &sorbit_layout_path()) {
            parse_meta_list_attr(
                attr,
                &mut [
                    ("offset", &mut |meta| parse_literal_int_meta(offset, meta)),
                    ("align", &mut |meta| parse_literal_int_meta(align, meta)),
                    ("round", &mut |meta| parse_literal_int_meta(round, meta)),
                ],
            )?;
        }

        Ok(attributes)
    }
}

impl Default for BitFieldAttribute {
    fn default() -> Self {
        Self { repr: placeholder_type(), offset: None, align: None, round: None }
    }
}

fn parse_bit_fields<'a>(
    attrs: impl Iterator<Item = &'a Attribute>,
) -> Result<HashMap<String, BitFieldAttribute>, syn::Error> {
    let mut bit_fields = HashMap::<String, BitFieldAttribute>::new();

    for attr in attrs.filter(|attr| attr.path() == &sorbit_bit_field_path()) {
        let meta_list = attr.meta.require_list()?;
        let metas = meta_list.parse_args_with(|parse_buffer: &syn::parse::ParseBuffer<'_>| {
            Punctuated::<Meta, Comma>::parse_terminated(parse_buffer)
        })?;
        if metas.is_empty() {
            continue;
        }
        let name_meta = metas[0].require_path_only()?;
        let name_ident = name_meta.get_ident().ok_or(syn::Error::new(name_meta.span(), "expected an identifier"))?;
        let name = name_ident.to_string();
        let attributes = bit_fields.entry(name.clone()).or_insert(BitFieldAttribute::default());

        let BitFieldAttribute { repr, offset, align, round } = attributes;
        parse_meta_list_attr(
            attr,
            &mut [
                (name.as_str(), &mut |meta| Ok(())), // Ignore the name parameter.
                ("repr", &mut |meta| parse_type_meta(repr, meta)),
                ("offset", &mut |meta| parse_literal_int_meta(offset, meta)),
                ("align", &mut |meta| parse_literal_int_meta(align, meta)),
                ("round", &mut |meta| parse_literal_int_meta(round, meta)),
            ],
        )?;
    }

    Ok(bit_fields)
}

/// Parse but ignore meta attributes.
///
/// [`syn`] needs us to parse a meta attribute fully, even if we don't care
/// about its contents. This is a bit of a hack to fix this.
fn ignore_nested_meta(meta: &ParseNestedMeta) -> Result<(), syn::Error> {
    let _ = meta.parse_nested_meta(|meta| ignore_nested_meta(&meta));
    let _ = meta.value();
    Ok(())
}

fn ignore_nested_meta_with_ident(meta: &ParseNestedMeta, idents: &[&str]) -> Result<(), syn::Error> {
    if let Some(ident) = meta.path.get_ident() {
        if idents.contains(&ident.to_string().as_str()) {
            ignore_nested_meta(&meta);
            return Ok(());
        }
    }
    return Err(meta.error("unrecognized parameter"));
}

#[cfg(test)]
mod tests {
    use super::*;

    use quote::quote;
    use syn::{Attribute, DeriveInput, parse_quote, parse_quote_spanned};

    //--------------------------------------------------------------------------
    // Struct attributes.
    //--------------------------------------------------------------------------

    #[test]
    fn parse_struct_attributes_none() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[derive(Clone)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: None, round: None });
        Ok(())
    }

    #[test]
    fn parse_struct_attributes_len() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(len=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: Some(4), round: None });
        Ok(())
    }

    #[test]
    fn parse_struct_attributes_pad() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(round=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: None, round: Some(4) });
        Ok(())
    }

    #[test]
    fn parse_struct_attributes_snap_and_pad_separate() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(len=8)]
            #[sorbit::layout(round=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: Some(8), round: Some(4) });
        Ok(())
    }

    #[test]
    fn parse_struct_attributes_snap_and_pad_together() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(len=8, round=4)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: Some(8), round: Some(4) });
        Ok(())
    }

    #[test]
    fn parse_struct_attributes_unrecognized() {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::layout(foo)]
        };
        assert!(StructAttribute::parse(input.iter()).is_err());
    }

    #[test]
    fn parse_struct_attributes_ignore_bit_field() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(a, b=0)]
        };
        let attrs = StructAttribute::parse(input.iter())?;
        assert_eq!(attrs, StructAttribute { len: None, round: None });
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Field attributes.
    //--------------------------------------------------------------------------

    //--------------------------------------------------------------------------
    // Bit field declarations.
    //--------------------------------------------------------------------------

    #[test]
    fn parse_bit_fields_minimal() -> Result<(), syn::Error> {
        let input: Vec<Attribute> = parse_quote! {
            #[sorbit::bit_field(A, repr(u8))]
        };
        let bit_fields = parse_bit_fields(input.iter())?;
        let expected = [(
            "A".to_string(),
            BitFieldAttribute { repr: parse_quote!(u8), offset: None, align: None, round: None },
        )]
        .into_iter()
        .collect();
        assert_eq!(bit_fields, expected);
        Ok(())
    }

    //--------------------------------------------------------------------------
    // Whole struct.
    //--------------------------------------------------------------------------

    #[test]
    fn parse_struct() -> Result<(), syn::Error> {
        let input: DeriveInput = parse_quote! {
            #[sorbit::layout(len=16)]
            #[sorbit::bit_field(A, ty=u8, round=4)]
            struct Test {
                #[sorbit::bit_field(A, bits=0..4)]
                foo: u8,
                #[sorbit::bit_field(A, bits=4..8)]
                bar: i8,
                #[sorbit::bit_field(A, bits=4..8)]
                baz: u32,
            }
        };
        Struct::parse(&input)?;
        Ok(())
    }
}
