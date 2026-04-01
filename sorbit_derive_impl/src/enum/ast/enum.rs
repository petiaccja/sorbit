use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{BinOp, Expr, ExprBinary, ExprLit, Generics, Ident, Lit, LitInt, Member, Pat, Token, Type, parse_quote};

use crate::attribute::ByteOrder;
use crate::r#enum::ast::variant::{CatchAll, Variant};
use crate::r#enum::parse;
use crate::ir::{Region, ToDeserializeOp, ToSerializeOp, Value};
use crate::ops::algorithm::with_maybe_byte_order;
use crate::ops::{
    self, custom_expr, declare_struct, deserialize_object, error, impl_deserialize, impl_serialize, match_, member, ok,
    ref_, self_, serialize_composite, serialize_object, struct_, symref, try_, use_,
};
use crate::r#struct::ast::Struct;
use crate::utility::{deconstruct_pattern_explicit, member_to_ident};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub ident: Ident,
    pub storage_ty: Type,
    pub generics: Generics,
    pub byte_order: Option<ByteOrder>,
    pub variants: Vec<Variant>,
}

impl Enum {
    pub fn is_multi_pass(&self) -> bool {
        self.variants
            .iter()
            .filter_map(|variant| variant.content.as_ref())
            .any(|content| content.is_multi_pass())
    }

    fn regular_variants(&self) -> impl Iterator<Item = &Variant> {
        self.variants.iter().filter(|variant| variant.catch_all == CatchAll::None)
    }

    fn catch_all_variants(&self) -> impl Iterator<Item = &Variant> {
        self.variants.iter().filter(|variant| variant.catch_all != CatchAll::None)
    }

    pub fn to_pack_into_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let storage_ty = &self.storage_ty;

        if self.variants.iter().any(|variant| variant.content.is_some()) {
            return syn::Error::new(
                ident.span(),
                "`PackInto` cannot be derived for enums with variants that have fields",
            )
            .into_compile_error();
        }

        let regular_arms = self.regular_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            quote! { Self::#ident => { ((#discr_expr) as #storage_ty).pack_into(num_bits) } }
        });
        let catch_all_arm = self.catch_all_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            match &variant.catch_all {
                CatchAll::None | CatchAll::Blanket => {
                    quote! { Self::#ident => { ((#discr_expr) as #storage_ty).pack_into(num_bits) } }
                }
                CatchAll::Discriminant(member) => match member {
                    Member::Named(ident) => quote! { Self::#ident{ #ident } => { #ident.pack_into(num_bits) } },
                    Member::Unnamed(_) => quote! { Self::#ident(discr) => { discr.pack_into(num_bits) } },
                },
            }
        });
        let arms = regular_arms.chain(catch_all_arm);

        quote! {
            impl<Packed> ::sorbit::bit::PackInto<Packed> for #ident
            where
                #storage_ty: ::sorbit::bit::PackInto<Packed>,
            {
                fn pack_into(&self, num_bits: usize) -> ::core::option::Option<Packed> {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
    }

    pub fn to_unpack_from_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let storage_ty = &self.storage_ty;

        if self.variants.iter().any(|variant| variant.content.is_some()) {
            return syn::Error::new(
                ident.span(),
                "`UnpackFrom` cannot be derived for enums with variants that have fields",
            )
            .into_compile_error();
        }

        let regular_arms = self.regular_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            quote! { n if n == (#discr_expr) as #storage_ty => { ::core::result::Result::Ok(Self::#ident) } }
        });
        let catch_all_arm = self.catch_all_variants().map(|variant| {
            let variant_ident = &variant.ident;
            match &variant.catch_all {
                CatchAll::None | CatchAll::Blanket => {
                    quote! { _ => { ::core::result::Result::Ok(Self::#variant_ident) } }
                }
                CatchAll::Discriminant(member) => match member {
                    Member::Named(catch_all_ident) => {
                        quote! { n => { ::core::result::Result::Ok(Self::#variant_ident{ #catch_all_ident: n }) } }
                    }
                    Member::Unnamed(_) => quote! { n => { ::core::result::Result::Ok(Self::#variant_ident(n)) } },
                },
            }
        });
        let unmatched_arm = (self.catch_all_variants().count() == 0).then(|| {
            quote! { _ => { Err(value) } }
        });
        let arms = regular_arms.chain(catch_all_arm).chain(unmatched_arm);

        quote! {
            impl<Packed> ::sorbit::bit::UnpackFrom<Packed> for #ident
            where
                #storage_ty: ::sorbit::bit::UnpackFrom<Packed>,
                Packed: Clone,
            {
                fn unpack_from(value: Packed, num_bits: usize) -> ::core::result::Result<Self, Packed> {
                    match u8::unpack_from(value.clone(), num_bits)? {
                        #(#arms)*
                    }
                }
            }
        }
    }
}

impl TryFrom<parse::Enum> for Enum {
    type Error = syn::Error;
    fn try_from(mut value: parse::Enum) -> Result<Self, Self::Error> {
        let storage_ty = value.storage_ty.unwrap_or(parse_quote!(isize));

        let catch_all_variants = value.variants.iter().filter(|variant| variant.catch_all != parse::CatchAll::None);
        if let Some(repeat_catch_all) = catch_all_variants.skip(1).next() {
            return Err(syn::Error::new(
                repeat_catch_all.ident.span(),
                "second catch_all variant defined here is not allowed, there must be zero or one catch_all variants",
            ));
        }

        let discriminants = compute_discriminants(value.variants.iter_mut().map(|variant| variant.discriminant.take()));
        let variants = std::iter::zip(value.variants.into_iter(), discriminants.into_iter())
            .map(|(variant, discriminant)| -> Result<Variant, syn::Error> {
                let catch_all = match variant.catch_all {
                    parse::CatchAll::None => CatchAll::None,
                    parse::CatchAll::Blanket => CatchAll::Blanket,
                    parse::CatchAll::Discriminant(member, ty) => {
                        if ty == storage_ty {
                            CatchAll::Discriminant(member)
                        } else {
                            return Err(syn::Error::new(ty.span(), "catch_all type must be the same as the enum repr"));
                        }
                    }
                };
                let content = variant.content.map(|content| Struct::try_from(content)).transpose()?;
                Ok(Variant { ident: variant.ident, discriminant, catch_all, content })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { ident: value.ident, storage_ty, generics: value.generics, byte_order: value.byte_order, variants })
    }
}

impl ToSerializeOp for Enum {
    type Args = ();
    fn to_serialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_serialize(
            region,
            self.ident.clone(),
            self.generics.clone(),
            self.is_multi_pass(),
            Region::build(|region, [serializer]| {
                let result = with_maybe_byte_order(region, serializer, self.byte_order, true, |region, serializer| {
                    let self_ = self_(region);
                    let arms = self
                        .variants
                        .iter()
                        .map(|variant| serialize_arm(&self.ident, &self.storage_ty, serializer, variant));
                    match_(region, self_, arms.collect())
                });
                vec![result]
            }),
        );
        vec![]
    }
}

impl ToDeserializeOp for Enum {
    type Args = ();
    fn to_deserialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_deserialize(
            region,
            self.ident.clone(),
            self.generics.clone(),
            Region::build(|region, [deserializer]| {
                let result =
                    with_maybe_byte_order(region, deserializer, self.byte_order, false, |region, deserializer| {
                        let maybe_discriminant = deserialize_object(region, deserializer, self.storage_ty.clone());
                        let discriminant = try_(region, maybe_discriminant);
                        let normal_arms =
                            self.regular_variants().map(|variant| deserialize_arm(&self.ident, variant, deserializer));
                        let catch_all_arm = self
                            .catch_all_variants()
                            .map(|variant| deserialize_arm(&self.ident, variant, deserializer));
                        let unmatched_arm =
                            (self.catch_all_variants().count() == 0).then(|| deserialize_unmatched_arm(deserializer));
                        let arms = normal_arms.chain(catch_all_arm).chain(unmatched_arm);
                        match_(region, discriminant, arms.collect())
                    });
                vec![result]
            }),
        );
        vec![]
    }
}

fn serialize_arm(
    self_ident: &Ident,
    storage_ty: &Type,
    serializer: Value,
    variant: &Variant,
) -> (syn::Pat, Option<Expr>, Region) {
    let pattern = serialize_arm_pattern(self_ident, variant);
    let content = variant.content.as_ref();
    let body = Region::build(move |region: &mut Region, []| {
        if let Some(content) = content {
            let result_comp = serialize_composite(
                region,
                serializer,
                Region::build(move |region, [serializer]| {
                    let discr_result = serialize_arm_discr(region, serializer, storage_ty, variant);
                    try_(region, discr_result);
                    let result = content.serialize_members(region, serializer);
                    vec![result]
                }),
            );
            let span_comp = try_(region, result_comp);
            let span_comp0 = member(region, span_comp, syn::Member::from(0), false);
            vec![ok(region, span_comp0)]
        } else {
            vec![serialize_arm_discr(region, serializer, storage_ty, variant)]
        }
    });
    (pattern, None, body)
}

fn serialize_arm_pattern(self_ident: &Ident, variant: &Variant) -> Pat {
    let variant_ident = &variant.ident;
    let member_offset = match &variant.catch_all {
        CatchAll::None => 0,
        CatchAll::Blanket => 0,
        CatchAll::Discriminant(_) => 1,
    };
    let mut pattern_members = Vec::new();
    match &variant.catch_all {
        CatchAll::None => (),
        CatchAll::Blanket => (),
        CatchAll::Discriminant(member) => pattern_members.push((member.clone(), format_ident!("discr"))),
    }
    match &variant.content {
        Some(content) => pattern_members.extend(content.members().iter().map(|member| {
            (
                match member {
                    Member::Named(ident) => Member::Named(ident.clone()),
                    Member::Unnamed(index) => Member::from((index.index + member_offset) as usize),
                },
                member_to_ident((*member).clone()),
            )
        })),
        None => (),
    }
    match pattern_members.len() {
        0 => parse_quote!(#self_ident::#variant_ident),
        _ => deconstruct_pattern_explicit(&parse_quote!(#self_ident::#variant_ident), pattern_members.into_iter()),
    }
}

fn serialize_arm_discr(region: &mut Region, serializer: Value, discr_ty: &Type, variant: &Variant) -> Value {
    let discr = match &variant.catch_all {
        CatchAll::None | CatchAll::Blanket => {
            let discr_expr = variant.discriminant.clone();
            let discr_cast = parse_quote!( (#discr_expr) as #discr_ty );
            let discr = custom_expr(region, discr_cast);
            ref_(region, discr)
        }
        CatchAll::Discriminant(_) => symref(region, parse_quote!(discr)),
    };

    serialize_object(region, serializer, discr, false)
}

fn deserialize_arm(self_ident: &Ident, variant: &Variant, deserializer: Value) -> (syn::Pat, Option<Expr>, Region) {
    let variant_ident = variant.ident.clone();
    let pat = parse_quote!(discr);
    let discr_expr = &variant.discriminant;
    let guard_expr = match &variant.catch_all {
        CatchAll::None => Some(parse_quote!(discr == #discr_expr)),
        CatchAll::Blanket => None,
        CatchAll::Discriminant(_) => None,
    };

    let struct_ty = parse_quote!(#self_ident::#variant_ident);
    let self_ident = self_ident.clone();
    let body = Region::build(move |region, []| {
        let result = match &variant.catch_all {
            CatchAll::None | CatchAll::Blanket => match &variant.content {
                Some(content) => {
                    use_(region, parse_quote!(#self_ident::#variant_ident));
                    content.deserialize_members(region, deserializer)
                }
                None => {
                    let value = struct_(region, struct_ty, vec![]);
                    ok(region, value)
                }
            },
            CatchAll::Discriminant(catch_all) => match &variant.content {
                Some(content) => {
                    let fields: Vec<_> = content.fields();
                    declare_struct(
                        region,
                        variant.ident.clone(),
                        fields.iter().map(|(m, t)| ((*m).clone(), (*t).clone())).collect(),
                    );
                    let discr = symref(region, parse_quote!(discr));
                    let content_result = content.deserialize_members(region, deserializer);
                    let content = try_(region, content_result);
                    let values = std::iter::once((catch_all.clone(), discr))
                        .chain(fields.iter().map(|(member, _)| {
                            let target_member = match member {
                                Member::Named(ident) => Member::from(ident.clone()),
                                Member::Unnamed(index) => Member::from((index.index + 1) as usize),
                            };
                            let value = ops::member(region, content, (*member).clone(), false);
                            (target_member, value)
                        }))
                        .collect();
                    let value = struct_(region, struct_ty, values);
                    ok(region, value)
                }
                None => {
                    let discr = symref(region, parse_quote!(discr));
                    let value = struct_(region, struct_ty, vec![(catch_all.clone(), discr)]);
                    ok(region, value)
                }
            },
        };
        vec![result]
    });
    (pat, guard_expr, body)
}

fn deserialize_unmatched_arm(deserializer: Value) -> (syn::Pat, Option<Expr>, Region) {
    let pat = parse_quote!(_);
    let body = Region::build(move |region: &mut Region, []| {
        vec![error(
            region,
            deserializer,
            "invalid enum discriminant".into(),
        )]
    });
    (pat, None, body)
}

fn compute_discriminants(variants: impl Iterator<Item = Option<Expr>>) -> Vec<Expr> {
    variants
        .scan((None, 0isize), |(prev, increment), current| match (&prev, current) {
            (_, Some(current)) => {
                *prev = Some(current.clone());
                *increment = 0;
                Some(current)
            }
            (Some(prev), None) => {
                *increment += 1;
                Some(Expr::Binary(ExprBinary {
                    attrs: vec![],
                    left: Box::new(prev.clone()),
                    op: BinOp::Add(Token![+](Span::call_site())),
                    right: Box::new(literal_int_expr(*increment)),
                }))
            }
            (None, None) => {
                *prev = Some(literal_int_expr(0));
                prev.clone()
            }
        })
        .collect()
}

fn literal_int_expr(value: isize) -> Expr {
    Expr::Lit(ExprLit { attrs: vec![], lit: Lit::Int(LitInt::new(&format!("{value}"), Span::call_site())) })
}

#[cfg(test)]
mod tests {
    use crate::attribute::Transform;
    use crate::ir::pattern_match::assert_matches;
    use crate::r#struct::ast::Field;

    use super::*;

    use syn::parse_quote;

    #[test]
    fn compute_discriminants_implicit() {
        let input = [None, None];
        let expected: [Expr; 2] = [parse_quote!(0), parse_quote!(0 + 1)];
        let result = compute_discriminants(input.into_iter());
        assert_eq!(result, expected);
    }

    #[test]
    fn compute_discriminants_explicit() {
        let input = [Some(parse_quote!(754)), None, Some(parse_quote!(854)), None];
        let expected: [Expr; 4] = [
            parse_quote!(754),
            parse_quote!(754 + 1),
            parse_quote!(854),
            parse_quote!(854 + 1),
        ];
        let result = compute_discriminants(input.into_iter());
        assert_eq!(result, expected);
    }

    fn create_simple() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: None,
                },
                Variant {
                    ident: parse_quote!(B),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::None,
                    content: None,
                },
            ],
        }
    }

    fn create_catch_all_empty() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: None,
                },
                Variant {
                    ident: parse_quote!(CatchAll),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::Blanket,
                    content: None,
                },
            ],
        }
    }

    fn create_catch_all_tuple() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: None,
                },
                Variant {
                    ident: parse_quote!(CatchAll),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::Discriminant(parse_quote!(0)),
                    content: None,
                },
            ],
        }
    }

    fn create_catch_all_struct() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: None,
                },
                Variant {
                    ident: parse_quote!(CatchAll),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::Discriminant(parse_quote!(ca)),
                    content: None,
                },
            ],
        }
    }

    fn create_catch_all_content_tuple() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: None,
                },
                Variant {
                    ident: parse_quote!(CatchAll),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::Discriminant(parse_quote!(0)),
                    content: Some(Struct {
                        ident: parse_quote!(CatchAll),
                        generics: Generics::default(),
                        byte_order: None,
                        len: None,
                        round: None,
                        fields: vec![Field::Direct {
                            member: parse_quote!(0),
                            ty: parse_quote!(i8),
                            multi_pass: None,
                            transform: Transform::None,
                            layout_properties: Default::default(),
                        }],
                    }),
                },
            ],
        }
    }

    fn create_catch_all_content_struct() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: None,
                },
                Variant {
                    ident: parse_quote!(CatchAll),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::Discriminant(parse_quote!(ca)),
                    content: Some(Struct {
                        ident: parse_quote!(CatchAll),
                        generics: Generics::default(),
                        byte_order: None,
                        len: None,
                        round: None,
                        fields: vec![Field::Direct {
                            member: parse_quote!(b),
                            ty: parse_quote!(i8),
                            multi_pass: None,
                            transform: Transform::None,
                            layout_properties: Default::default(),
                        }],
                    }),
                },
            ],
        }
    }

    fn create_fielded() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
                    catch_all: CatchAll::None,
                    content: Some(Struct {
                        ident: parse_quote!(A),
                        generics: Generics::default(),
                        byte_order: None,
                        len: None,
                        round: None,
                        fields: vec![Field::Direct {
                            member: parse_quote!(0),
                            ty: parse_quote!(u8),
                            multi_pass: None,
                            transform: Transform::None,
                            layout_properties: Default::default(),
                        }],
                    }),
                },
                Variant {
                    ident: parse_quote!(B),
                    discriminant: parse_quote!(1),
                    catch_all: CatchAll::None,
                    content: Some(Struct {
                        ident: parse_quote!(B),
                        generics: Generics::default(),
                        byte_order: None,
                        len: None,
                        round: None,
                        fields: vec![Field::Direct {
                            member: parse_quote!(b),
                            ty: parse_quote!(i8),
                            multi_pass: None,
                            transform: Transform::None,
                            layout_properties: Default::default(),
                        }],
                    }),
                },
            ],
        }
    }

    #[test]
    fn to_serialize_op_simple() {
        let input = create_simple();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object [false] %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: B => {
                        %disc_b = custom_expr [(1) as u16]
                        %disc_b_ref = ref %disc_b
                        %result_b = serialize_object [false] %serializer, %disc_b_ref
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_simple() {
        let input = create_simple();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discr if discr == 1 => {
                        %result_b = struct [Test::B]
                        %result_b_ok = ok %result_b
                        yield %result_b_ok
                    }
                    _ => {
                        %result_err = error [invalid enum discriminant] %deserializer
                        yield %result_err
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_empty() {
        let input = create_catch_all_empty();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object [false] %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll => {
                        %disc_b = custom_expr [(1) as u16]
                        %disc_b_ref = ref %disc_b
                        %result_b = serialize_object [false] %serializer, %disc_b_ref
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_tuple() {
        let input = create_catch_all_tuple();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object [false] %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll { 0 : discr } => {
                        %disc_ca = symref [discr]
                        %result_ca = serialize_object [false] %serializer, %disc_ca
                        yield %result_ca
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_struct() {
        let input = create_catch_all_struct();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object [false] %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll { ca : discr } => {
                        %disc_ca = symref [discr]
                        %result_ca = serialize_object [false] %serializer, %disc_ca
                        yield %result_ca
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_content_tuple() {
        let input = create_catch_all_content_tuple();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object [false] %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll { 0 : discr, 1 : m0 } => {
                        %result_comp_b = serialize_composite %serializer |%se_inner_b| {
                            %disc_ca = symref [discr]
                            %result_ca = serialize_object [false] %se_inner_b, %disc_ca
                            %span_ca = try %result_ca

                            %result_cont_b = serialize_composite %se_inner_b |%se_cont_b| {
                                %m0 = symref [m0]
                                %maybe_span_m0 = serialize_object [false] %se_cont_b, %m0

                                %span_m0 = try %maybe_span_m0
                                %spans_b = tuple %span_m0
                                %result_spans_b = ok %spans_b
                                yield %result_spans_b
                            }
                            %span_cont_b = try %result_cont_b
                            %span_cont_b0 = member [0, false] %span_cont_b
                            %result_cont_b0 = ok %span_cont_b0
                            yield %result_cont_b0
                        }
                        %span_comp_b = try %result_comp_b
                        %span_all_b = member [0, false] %span_comp_b
                        %result_b = ok %span_all_b
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_content_struct() {
        let input = create_catch_all_content_struct();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object [false] %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll { ca : discr, b } => {
                        %result_comp_b = serialize_composite %serializer |%se_inner_b| {
                            %disc_ca = symref [discr]
                            %result_ca = serialize_object [false] %se_inner_b, %disc_ca
                            %span_ca = try %result_ca

                            %result_cont_b = serialize_composite %se_inner_b |%se_cont_b| {
                                %b = symref [b]
                                %maybe_span_b = serialize_object [false] %se_cont_b, %b

                                %span_b = try %maybe_span_b
                                %spans_b = tuple %span_b
                                %result_spans_b = ok %spans_b
                                yield %result_spans_b
                            }
                            %span_cont_b = try %result_cont_b
                            %span_cont_b0 = member [0, false] %span_cont_b
                            %result_cont_b0 = ok %span_cont_b0
                            yield %result_cont_b0
                        }
                        %span_comp_b = try %result_comp_b
                        %span_all_b = member [0, false] %span_comp_b
                        %result_b = ok %span_all_b
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_empty() {
        let input = create_catch_all_empty();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discr => {
                        %result_ca = struct [Test::CatchAll]
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_tuple() {
        let input = create_catch_all_tuple();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discr => {
                        %discr_pat = symref [discr]
                        %result_ca = struct [Test::CatchAll, 0] %discr_pat
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_struct() {
        let input = create_catch_all_struct();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discr => {
                        %discr_pat = symref [discr]
                        %result_ca = struct [Test::CatchAll, ca] %discr_pat
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_content_tuple() {
        let input = create_catch_all_content_tuple();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        println!("{}", region.to_token_stream_formatted(false));
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discr => {
                        declare_struct [CatchAll, 0: i8 ]

                        %discr_pat = symref [discr]
                        
                        %result_temp_struct = deserialize_composite %deserializer |%de_temp_struct| {
                            %result_m0 = deserialize_object [i8] %de_temp_struct
                            %m0 = try %result_m0
                            sym [m0] %m0
                            %struct_b = struct [CatchAll, 0] %m0
                            %result_struct_b = ok %struct_b
                            yield %result_struct_b
                        }
                        %temp_struct = try %result_temp_struct
                        %temp_struct_b = member [0, false] %temp_struct
                            
                        %result_ca = struct [Test::CatchAll, 0, 1] %discr_pat %temp_struct_b
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_content_struct() {
        let input = create_catch_all_content_struct();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discr => {
                        declare_struct [CatchAll, b: i8 ]

                        %discr_pat = symref [discr]
                        
                        %result_temp_struct = deserialize_composite %deserializer |%de_temp_struct| {
                            %result_b = deserialize_object [i8] %de_temp_struct
                            %b = try %result_b
                            sym [b] %b
                            %struct_b = struct [CatchAll, b] %b
                            %result_struct_b = ok %struct_b
                            yield %result_struct_b
                        }
                        %temp_struct = try %result_temp_struct
                        %temp_struct_b = member [b, false] %temp_struct
                            
                        %result_ca = struct [Test::CatchAll, ca, b] %discr_pat %temp_struct_b
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_fielded() {
        let input = create_fielded();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, false ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A { 0 : m0 } => {
                        %result_comp_a = serialize_composite %serializer |%se_inner_a| {
                            %disc_a = custom_expr [(0) as u16]
                            %disc_a_ref = ref %disc_a
                            %result_disc_a = serialize_object [false] %se_inner_a, %disc_a_ref
                            %span_disc_a = try %result_disc_a

                            %result_cont_a = serialize_composite %se_inner_a |%se_cont_a| {
                                %m0 = symref [m0]
                                %maybe_span_m0 = serialize_object [false] %se_cont_a, %m0

                                %span_m0 = try %maybe_span_m0
                                %spans_a = tuple %span_m0
                                %result_spans_a = ok %spans_a
                                yield %result_spans_a
                            }
                            %span_cont_a = try %result_cont_a
                            %span_cont_a0 = member [0, false] %span_cont_a
                            %result_cont_a0 = ok %span_cont_a0
                            yield %result_cont_a0
                        }
                        %span_comp_a = try %result_comp_a
                        %span_all_a = member [0, false] %span_comp_a
                        %result_a = ok %span_all_a
                        yield %result_a
                    }
                    Test :: B { b } => {
                        %result_comp_b = serialize_composite %serializer |%se_inner_b| {
                            %disc_b = custom_expr [(1) as u16]
                            %disc_b_ref = ref %disc_b
                            %result_disc_b = serialize_object [false] %se_inner_b, %disc_b_ref
                            %span_disc_b = try %result_disc_b

                            %result_cont_b = serialize_composite %se_inner_b |%se_cont_b| {
                                %b = symref [b]
                                %maybe_span_b = serialize_object [false] %se_cont_b, %b

                                %span_b = try %maybe_span_b
                                %spans_b = tuple %span_b
                                %result_spans_b = ok %spans_b
                                yield %result_spans_b
                            }
                            %span_cont_b = try %result_cont_b
                            %span_cont_b0 = member [0, false] %span_cont_b
                            %result_cont_b0 = ok %span_cont_b0
                            yield %result_cont_b0
                        }
                        %span_comp_b = try %result_comp_b
                        %span_all_b = member [0, false] %span_comp_b
                        %result_b = ok %span_all_b
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_fielded() {
        let input = create_fielded();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discr if discr == 0 => {
                        use [Test::A]
                        %result_cont_a = deserialize_composite %deserializer |%de_cont_a| {
                            %result_m0 = deserialize_object [u8] %de_cont_a
                            %m0 = try %result_m0
                            sym [m0] %m0
                            %struct_a = struct [A, 0] %m0
                            %result_struct_a = ok %struct_a
                            yield %result_struct_a
                        }
                        yield %result_cont_a
                    }
                    discr if discr == 1 => {
                        use [Test::B]
                        %result_cont_b = deserialize_composite %deserializer |%de_cont_b| {
                            %result_b = deserialize_object [i8] %de_cont_b
                            %b = try %result_b
                            sym [b] %b
                            %struct_b = struct [B, b] %b
                            %result_struct_b = ok %struct_b
                            yield %result_struct_b
                        }
                        yield %result_cont_b
                    }
                    _ => {
                        %result_err = error [invalid enum discriminant] %deserializer
                        yield %result_err
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }
}
