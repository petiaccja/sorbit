use std::collections::HashSet;
use std::ops::Range;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{Ident, Member, Type};

use super::super::parse;
use super::field::Field;
use crate::attribute::{BitNumbering, ByteOrder, Transform};
use crate::r#struct::ast::field::BitFieldMember;
use crate::r#struct::parse::{BitFieldStorageProperties, FieldLayoutProperties};
use crate::utility::to_member;

pub fn group_fields(fields: impl Iterator<Item = parse::Field>) -> Result<Vec<LayoutField>, syn::Error> {
    let mut layout_fields = Vec::new();
    let mut layout_field_idents = HashSet::new();

    for (index, field) in fields.enumerate() {
        match field {
            parse::Field::Direct { ident, ty, transform: value, layout_properties } => {
                let member = to_member(ident, index, ty.span());
                layout_fields.push(LayoutField::Direct { member, ty, transform: value, layout_properties });
            }
            parse::Field::Bit { ident, ty, transform, bits, storage_ident, storage_properties, layout_properties } => {
                let member = to_member(ident, index, ty.span());
                match layout_fields.last_mut() {
                    Some(LayoutField::Bit { ident, sub_fields }) if *ident == storage_ident => {
                        let sub_field =
                            LayoutSubField { member, ty, transform, bits, storage_properties, layout_properties };
                        sub_fields.push(sub_field);
                    }
                    _ => {
                        let sub_field =
                            LayoutSubField { member, ty, transform, bits, storage_properties, layout_properties };
                        if layout_field_idents.insert(storage_ident.clone()) {
                            layout_fields.push(LayoutField::Bit { ident: storage_ident, sub_fields: vec![sub_field] });
                        } else {
                            return Err(syn::Error::new(
                                sub_field.member.span(),
                                format!("the members of bit field `{}` must be consecutive", storage_ident),
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(layout_fields)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutField {
    Direct { member: Member, ty: Type, transform: Transform, layout_properties: FieldLayoutProperties },
    Bit { ident: Ident, sub_fields: Vec<LayoutSubField> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutSubField {
    member: Member,
    ty: Type,
    transform: Transform,
    bits: Range<u8>,
    storage_properties: BitFieldStorageProperties,
    layout_properties: FieldLayoutProperties,
}

impl LayoutField {
    pub fn into_field(self) -> Result<Field, syn::Error> {
        match self {
            LayoutField::Direct { member, ty, transform, layout_properties } => {
                Ok(Field::Direct { member, ty, transform, layout_properties })
            }
            LayoutField::Bit { ident, sub_fields } => {
                let ty = Self::find_storage_ty(sub_fields.iter(), ident.span())?;
                let bit_numbering = Self::find_bit_numbering(sub_fields.iter())?.unwrap_or(BitNumbering::LSB0);

                let byte_order = Self::find_byte_order(sub_fields.iter())?;
                let offset = Self::find_offset(sub_fields.iter())?;
                let align = Self::find_align(sub_fields.iter())?;
                let round = Self::find_round(sub_fields.iter())?;
                let layout_properties = FieldLayoutProperties { byte_order, offset, align, round };

                let members = sub_fields
                    .into_iter()
                    .map(|LayoutSubField { member, ty, transform, bits, .. }| BitFieldMember {
                        member,
                        ty,
                        transform,
                        bits,
                    })
                    .collect();
                Ok(Field::Bit { ident, ty, bit_numbering, layout_properties, members })
            }
        }
    }

    fn find_storage_ty<'a>(items: impl Iterator<Item = &'a LayoutSubField>, span: Span) -> Result<Type, syn::Error> {
        let iter = items.filter_map(|item| item.storage_properties.storage_ty.as_ref().map(|ty| (ty, ty.span())));
        let ty = all_same_or_error(iter, "the storage type of the bit field is redefined with a different value")?;
        ty.cloned().ok_or(syn::Error::new(span, "the storage type of the bit field is not specified"))
    }

    fn find_byte_order<'a>(items: impl Iterator<Item = &'a LayoutSubField>) -> Result<Option<ByteOrder>, syn::Error> {
        let iter = items
            .filter_map(|item| item.layout_properties.byte_order.map(|byte_order| (byte_order, item.member.span())));
        all_same_or_error(iter, "the byte order of the bit field is redefined with a different value")
    }

    fn find_bit_numbering<'a>(
        items: impl Iterator<Item = &'a LayoutSubField>,
    ) -> Result<Option<BitNumbering>, syn::Error> {
        let iter = items.filter_map(|item| {
            item.storage_properties.bit_numbering.map(|bit_numbering| (bit_numbering, item.member.span()))
        });
        all_same_or_error(iter, "the bit numbering of the bit field is redefined with a different value")
    }

    fn find_offset<'a>(items: impl Iterator<Item = &'a LayoutSubField>) -> Result<Option<u64>, syn::Error> {
        let iter = items.filter_map(|item| item.layout_properties.offset.map(|offset| (offset, item.member.span())));
        all_same_or_error(iter, "the offset of the bit field is redefined with a different value")
    }

    fn find_align<'a>(items: impl Iterator<Item = &'a LayoutSubField>) -> Result<Option<u64>, syn::Error> {
        let iter = items.filter_map(|item| item.layout_properties.align.map(|align| (align, item.member.span())));
        all_same_or_error(iter, "alignment of the bit field is redefined with a different value")
    }

    fn find_round<'a>(items: impl Iterator<Item = &'a LayoutSubField>) -> Result<Option<u64>, syn::Error> {
        let iter = items.filter_map(|item| item.layout_properties.round.map(|round| (round, item.member.span())));
        all_same_or_error(iter, "rounding of the bit field is redefined with a different value")
    }
}

fn all_same_or_error<T: PartialEq>(
    mut iter: impl Iterator<Item = (T, Span)>,
    message: &str,
) -> Result<Option<T>, syn::Error> {
    let Some((value, _)) = iter.next() else {
        return Ok(None);
    };
    if let Some((_, span)) = iter.find(|(maybe_different, _)| *maybe_different != value) {
        Err(syn::Error::new(span, message))
    } else {
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod group_fields {
        use syn::parse_quote;

        use crate::attribute::Transform;

        use super::*;

        #[test]
        fn mixed_direct_and_bit() {
            let fields = [
                parse::Field::Direct {
                    ident: Some(parse_quote!(foo)),
                    ty: parse_quote!(u8),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                },
                parse::Field::Bit {
                    ident: parse_quote!(bar),
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    bits: 0..4,
                    storage_ident: parse_quote!(_bit_field),
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
                parse::Field::Direct {
                    ident: None,
                    ty: parse_quote!(u32),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                },
            ];
            let actual = group_fields(fields.into_iter()).unwrap();
            let expected = [
                LayoutField::Direct {
                    member: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                },
                LayoutField::Bit {
                    ident: parse_quote!(_bit_field),
                    sub_fields: vec![LayoutSubField {
                        member: parse_quote!(bar),
                        ty: parse_quote!(f32),
                        transform: Transform::None,
                        bits: 0..4,
                        storage_properties: Default::default(),
                        layout_properties: Default::default(),
                    }],
                },
                LayoutField::Direct {
                    member: parse_quote!(2),
                    ty: parse_quote!(u32),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                },
            ];
            assert_eq!(actual, expected);
        }

        #[test]
        fn split_by_storage_id() {
            let fields = [
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    storage_ident: parse_quote!(_bit_field_1),
                    bits: 0..4,
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    storage_ident: parse_quote!(_bit_field_1),
                    bits: 0..4,
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    storage_ident: parse_quote!(_bit_field_2),
                    bits: 0..4,
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
            ];
            let actual = group_fields(fields.into_iter()).unwrap();
            let expected = [
                LayoutField::Bit {
                    ident: parse_quote!(_bit_field_1),
                    sub_fields: vec![
                        LayoutSubField {
                            member: parse_quote!(0),
                            ty: parse_quote!(f32),
                            transform: Transform::None,
                            bits: 0..4,
                            storage_properties: Default::default(),
                            layout_properties: Default::default(),
                        },
                        LayoutSubField {
                            member: parse_quote!(1),
                            ty: parse_quote!(f32),
                            transform: Transform::None,
                            bits: 0..4,
                            storage_properties: Default::default(),
                            layout_properties: Default::default(),
                        },
                    ],
                },
                LayoutField::Bit {
                    ident: parse_quote!(_bit_field_2),
                    sub_fields: vec![LayoutSubField {
                        member: parse_quote!(2),
                        ty: parse_quote!(f32),
                        transform: Transform::None,
                        bits: 0..4,
                        storage_properties: Default::default(),
                        layout_properties: Default::default(),
                    }],
                },
            ];
            assert_eq!(actual, expected);
        }

        #[test]
        #[should_panic]
        fn non_consecutive_items() {
            let fields = [
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    storage_ident: parse_quote!(_bit_field_1),
                    bits: 0..4,
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    storage_ident: parse_quote!(_bit_field_2),
                    bits: 0..4,
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    transform: Transform::None,
                    storage_ident: parse_quote!(_bit_field_1),
                    bits: 0..4,
                    storage_properties: Default::default(),
                    layout_properties: Default::default(),
                },
            ];
            group_fields(fields.into_iter()).unwrap();
        }
    }

    mod field_group {
        use syn::parse_quote;

        use super::*;

        fn make_items() -> [LayoutSubField; 3] {
            std::array::from_fn(|index| LayoutSubField {
                member: index.into(),
                ty: parse_quote!(f32),
                transform: Transform::None,
                bits: 0..4,
                storage_properties: Default::default(),
                layout_properties: Default::default(),
            })
        }

        #[test]
        fn find_storage_ty_none() {
            let items = make_items();
            assert!(LayoutField::find_storage_ty(items.iter(), Span::call_site()).is_err());
        }

        #[test]
        fn find_storage_ty_unique() {
            let mut items = make_items();
            items[1].storage_properties.storage_ty = Some(parse_quote!(u32));
            items[2].storage_properties.storage_ty = Some(parse_quote!(u32));
            assert_eq!(LayoutField::find_storage_ty(items.iter(), Span::call_site()).unwrap(), parse_quote!(u32));
        }

        #[test]
        fn find_storage_ty_ambiguous() {
            let mut items = make_items();
            items[0].storage_properties.storage_ty = Some(parse_quote!(u32));
            items[2].storage_properties.storage_ty = Some(parse_quote!(u16));
            assert!(LayoutField::find_storage_ty(items.iter(), Span::call_site()).is_err());
        }

        #[test]
        fn find_offset_none() {
            let items = make_items();
            assert_eq!(LayoutField::find_offset(items.iter()).unwrap(), None);
        }

        #[test]
        fn find_offset_unique() {
            let mut items = make_items();
            items[1].layout_properties.offset = Some(1);
            assert_eq!(LayoutField::find_offset(items.iter()).unwrap(), Some(1));
        }

        #[test]
        fn find_offset_ambiguous() {
            let mut items = make_items();
            items[0].layout_properties.offset = Some(1);
            items[2].layout_properties.offset = Some(2);
            assert!(LayoutField::find_offset(items.iter()).is_err());
        }

        #[test]
        fn find_align_none() {
            let items = make_items();
            assert_eq!(LayoutField::find_align(items.iter()).unwrap(), None);
        }

        #[test]
        fn find_align_unique() {
            let mut items = make_items();
            items[1].layout_properties.align = Some(1);
            assert_eq!(LayoutField::find_align(items.iter()).unwrap(), Some(1));
        }

        #[test]
        fn find_align_ambiguous() {
            let mut items = make_items();
            items[0].layout_properties.align = Some(1);
            items[2].layout_properties.align = Some(2);
            assert!(LayoutField::find_align(items.iter()).is_err());
        }

        #[test]
        fn find_round_none() {
            let items = make_items();
            assert_eq!(LayoutField::find_round(items.iter()).unwrap(), None);
        }

        #[test]
        fn find_round_unique() {
            let mut items = make_items();
            items[1].layout_properties.round = Some(1);
            assert_eq!(LayoutField::find_round(items.iter()).unwrap(), Some(1));
        }

        #[test]
        fn find_round_ambiguous() {
            let mut items = make_items();
            items[0].layout_properties.round = Some(1);
            items[2].layout_properties.round = Some(2);
            assert!(LayoutField::find_round(items.iter()).is_err());
        }
    }
}
