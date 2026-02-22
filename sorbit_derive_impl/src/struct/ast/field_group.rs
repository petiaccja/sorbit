use std::collections::HashSet;
use std::ops::Range;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{Ident, Index, Member, Type};

use crate::attribute::{BitNumbering, ByteOrder};
use crate::r#struct::ast::field::BitFieldMember;

use super::super::parse;
use super::field::Field;

pub fn group_fields(fields: impl Iterator<Item = parse::Field>) -> Result<Vec<FieldGroup>, syn::Error> {
    let mut field_groups = Vec::new();
    let mut field_group_ids = HashSet::new();
    for (index, field) in fields.enumerate() {
        match field {
            parse::Field::Direct { ident, ty, byte_order, offset, align, round } => {
                let member = ident
                    .map(|ident| Member::from(ident))
                    .unwrap_or(Member::Unnamed(Index { index: index as u32, span: ty.span() }));
                field_groups.push(FieldGroup::Direct { member, ty, byte_order, offset, align, round });
            }
            parse::Field::Bit {
                ident,
                ty,
                bits,
                storage_id,
                storage_ty,
                byte_order,
                bit_numbering,
                offset,
                align,
                round,
            } => {
                let member = ident
                    .map(|ident| Member::from(ident))
                    .unwrap_or(Member::Unnamed(Index { index: index as u32, span: ty.span() }));
                match field_groups.last_mut() {
                    Some(FieldGroup::Bit { ident, items }) if *ident == storage_id => {
                        let item = FieldGroupBitItem {
                            member,
                            member_ty: ty,
                            bits,
                            byte_order,
                            bit_numbering,
                            storage_ty,
                            offset,
                            align,
                            round,
                        };
                        items.push(item);
                    }
                    _ => {
                        let item = FieldGroupBitItem {
                            member,
                            member_ty: ty,
                            bits,
                            byte_order,
                            bit_numbering,
                            storage_ty,
                            offset,
                            align,
                            round,
                        };
                        if field_group_ids.insert(storage_id.clone()) {
                            field_groups.push(FieldGroup::Bit { ident: storage_id, items: vec![item] });
                        } else {
                            return Err(syn::Error::new(
                                item.member.span(),
                                format!("the members of bit field `{}` must be consecutive", storage_id),
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(field_groups)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldGroup {
    Direct {
        member: Member,
        ty: Type,
        byte_order: Option<ByteOrder>,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
    Bit {
        ident: Ident,
        items: Vec<FieldGroupBitItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldGroupBitItem {
    member: Member,
    member_ty: Type,
    bits: Range<u8>,
    storage_ty: Option<Type>,
    byte_order: Option<ByteOrder>,
    bit_numbering: Option<BitNumbering>,
    offset: Option<u64>,
    align: Option<u64>,
    round: Option<u64>,
}

impl FieldGroup {
    pub fn into_field(self) -> Result<Field, syn::Error> {
        match self {
            FieldGroup::Direct { member, ty, byte_order, offset, align, round } => {
                Ok(Field::Direct { member, ty, byte_order, offset, align, round })
            }
            FieldGroup::Bit { ident, items } => {
                let ty = Self::find_storage_ty(items.iter(), ident.span())?;
                let byte_order = Self::find_byte_order(items.iter())?;
                let bit_numbering = Self::find_bit_numbering(items.iter())?.unwrap_or(BitNumbering::LSB0);
                let offset = Self::find_offset(items.iter())?;
                let align = Self::find_align(items.iter())?;
                let round = Self::find_round(items.iter())?;
                let members = items
                    .into_iter()
                    .map(|item| BitFieldMember { member: item.member, ty: item.member_ty, bits: item.bits })
                    .collect();
                Ok(Field::Bit { ident, ty, byte_order, bit_numbering, offset, align, round, members })
            }
        }
    }

    fn find_storage_ty<'a>(items: impl Iterator<Item = &'a FieldGroupBitItem>, span: Span) -> Result<Type, syn::Error> {
        let iter = items.filter_map(|item| item.storage_ty.as_ref().map(|ty| (ty, ty.span())));
        let ty = all_same_or_error(iter, "the storage type of the bit field is redefined with a different value")?;
        ty.cloned().ok_or(syn::Error::new(span, "the storage type of the bit field is not specified"))
    }

    fn find_byte_order<'a>(
        items: impl Iterator<Item = &'a FieldGroupBitItem>,
    ) -> Result<Option<ByteOrder>, syn::Error> {
        let iter = items.filter_map(|item| item.byte_order.map(|byte_order| (byte_order, item.member.span())));
        all_same_or_error(iter, "the byte order of the bit field is redefined with a different value")
    }

    fn find_bit_numbering<'a>(
        items: impl Iterator<Item = &'a FieldGroupBitItem>,
    ) -> Result<Option<BitNumbering>, syn::Error> {
        let iter = items.filter_map(|item| item.bit_numbering.map(|bit_numbering| (bit_numbering, item.member.span())));
        all_same_or_error(iter, "the bit numbering of the bit field is redefined with a different value")
    }

    fn find_offset<'a>(items: impl Iterator<Item = &'a FieldGroupBitItem>) -> Result<Option<u64>, syn::Error> {
        let iter = items.filter_map(|item| item.offset.map(|offset| (offset, item.member.span())));
        all_same_or_error(iter, "the offset of the bit field is redefined with a different value")
    }

    fn find_align<'a>(items: impl Iterator<Item = &'a FieldGroupBitItem>) -> Result<Option<u64>, syn::Error> {
        let iter = items.filter_map(|item| item.align.map(|align| (align, item.member.span())));
        all_same_or_error(iter, "alignment of the bit field is redefined with a different value")
    }

    fn find_round<'a>(items: impl Iterator<Item = &'a FieldGroupBitItem>) -> Result<Option<u64>, syn::Error> {
        let iter = items.filter_map(|item| item.round.map(|round| (round, item.member.span())));
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

        use super::*;

        #[test]
        fn mixed_direct_and_bit() {
            let fields = [
                parse::Field::Direct {
                    ident: Some(parse_quote!(foo)),
                    ty: parse_quote!(u8),
                    byte_order: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                parse::Field::Bit {
                    ident: parse_quote!(bar),
                    ty: parse_quote!(f32),
                    bits: 0..4,
                    storage_id: parse_quote!(_bit_field),
                    storage_ty: None,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                parse::Field::Direct {
                    ident: None,
                    ty: parse_quote!(u32),
                    byte_order: None,
                    offset: None,
                    align: None,
                    round: None,
                },
            ];
            let actual = group_fields(fields.into_iter()).unwrap();
            let expected = [
                FieldGroup::Direct {
                    member: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    byte_order: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                FieldGroup::Bit {
                    ident: parse_quote!(_bit_field),
                    items: vec![FieldGroupBitItem {
                        member: parse_quote!(bar),
                        member_ty: parse_quote!(f32),
                        bits: 0..4,
                        storage_ty: None,
                        byte_order: None,
                        bit_numbering: None,
                        offset: None,
                        align: None,
                        round: None,
                    }],
                },
                FieldGroup::Direct {
                    member: parse_quote!(2),
                    ty: parse_quote!(u32),
                    byte_order: None,
                    offset: None,
                    align: None,
                    round: None,
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
                    storage_id: parse_quote!(_bit_field_1),
                    storage_ty: None,
                    bits: 0..4,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    storage_id: parse_quote!(_bit_field_1),
                    storage_ty: None,
                    bits: 0..4,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    storage_id: parse_quote!(_bit_field_2),
                    storage_ty: None,
                    bits: 0..4,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
            ];
            let actual = group_fields(fields.into_iter()).unwrap();
            let expected = [
                FieldGroup::Bit {
                    ident: parse_quote!(_bit_field_1),
                    items: vec![
                        FieldGroupBitItem {
                            member: parse_quote!(0),
                            member_ty: parse_quote!(f32),
                            bits: 0..4,
                            storage_ty: None,
                            byte_order: None,
                            bit_numbering: None,
                            offset: None,
                            align: None,
                            round: None,
                        },
                        FieldGroupBitItem {
                            member: parse_quote!(1),
                            member_ty: parse_quote!(f32),
                            bits: 0..4,
                            storage_ty: None,
                            byte_order: None,
                            bit_numbering: None,
                            offset: None,
                            align: None,
                            round: None,
                        },
                    ],
                },
                FieldGroup::Bit {
                    ident: parse_quote!(_bit_field_2),
                    items: vec![FieldGroupBitItem {
                        member: parse_quote!(2),
                        member_ty: parse_quote!(f32),
                        bits: 0..4,
                        storage_ty: None,
                        byte_order: None,
                        bit_numbering: None,
                        offset: None,
                        align: None,
                        round: None,
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
                    storage_id: parse_quote!(_bit_field_1),
                    storage_ty: None,
                    bits: 0..4,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    storage_id: parse_quote!(_bit_field_2),
                    storage_ty: None,
                    bits: 0..4,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                parse::Field::Bit {
                    ident: None,
                    ty: parse_quote!(f32),
                    storage_id: parse_quote!(_bit_field_1),
                    storage_ty: None,
                    bits: 0..4,
                    byte_order: None,
                    bit_numbering: None,
                    offset: None,
                    align: None,
                    round: None,
                },
            ];
            group_fields(fields.into_iter()).unwrap();
        }
    }

    mod field_group {
        use syn::parse_quote;

        use super::*;

        fn make_items() -> [FieldGroupBitItem; 3] {
            std::array::from_fn(|index| FieldGroupBitItem {
                member: index.into(),
                member_ty: parse_quote!(f32),
                bits: 0..4,
                storage_ty: None,
                byte_order: None,
                bit_numbering: None,
                offset: None,
                align: None,
                round: None,
            })
        }

        #[test]
        fn find_storage_ty_none() {
            let items = make_items();
            assert!(FieldGroup::find_storage_ty(items.iter(), Span::call_site()).is_err());
        }

        #[test]
        fn find_storage_ty_unique() {
            let mut items = make_items();
            items[1].storage_ty = Some(parse_quote!(u32));
            items[2].storage_ty = Some(parse_quote!(u32));
            assert_eq!(FieldGroup::find_storage_ty(items.iter(), Span::call_site()).unwrap(), parse_quote!(u32));
        }

        #[test]
        fn find_storage_ty_ambiguous() {
            let mut items = make_items();
            items[0].storage_ty = Some(parse_quote!(u32));
            items[2].storage_ty = Some(parse_quote!(u16));
            assert!(FieldGroup::find_storage_ty(items.iter(), Span::call_site()).is_err());
        }

        #[test]
        fn find_offset_none() {
            let items = make_items();
            assert_eq!(FieldGroup::find_offset(items.iter()).unwrap(), None);
        }

        #[test]
        fn find_offset_unique() {
            let mut items = make_items();
            items[1].offset = Some(1);
            assert_eq!(FieldGroup::find_offset(items.iter()).unwrap(), Some(1));
        }

        #[test]
        fn find_offset_ambiguous() {
            let mut items = make_items();
            items[0].offset = Some(1);
            items[2].offset = Some(2);
            assert!(FieldGroup::find_offset(items.iter()).is_err());
        }

        #[test]
        fn find_align_none() {
            let items = make_items();
            assert_eq!(FieldGroup::find_align(items.iter()).unwrap(), None);
        }

        #[test]
        fn find_align_unique() {
            let mut items = make_items();
            items[1].align = Some(1);
            assert_eq!(FieldGroup::find_align(items.iter()).unwrap(), Some(1));
        }

        #[test]
        fn find_align_ambiguous() {
            let mut items = make_items();
            items[0].align = Some(1);
            items[2].align = Some(2);
            assert!(FieldGroup::find_align(items.iter()).is_err());
        }

        #[test]
        fn find_round_none() {
            let items = make_items();
            assert_eq!(FieldGroup::find_round(items.iter()).unwrap(), None);
        }

        #[test]
        fn find_round_unique() {
            let mut items = make_items();
            items[1].round = Some(1);
            assert_eq!(FieldGroup::find_round(items.iter()).unwrap(), Some(1));
        }

        #[test]
        fn find_round_ambiguous() {
            let mut items = make_items();
            items[0].round = Some(1);
            items[2].round = Some(2);
            assert!(FieldGroup::find_round(items.iter()).is_err());
        }
    }
}
