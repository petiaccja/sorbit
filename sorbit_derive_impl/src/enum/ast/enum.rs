use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned as _;
use syn::{BinOp, Expr, ExprBinary, ExprLit, Generics, Ident, Lit, LitInt, Token, Type, parse_quote};

use crate::attribute::ByteOrder;
use crate::r#enum::ast::variant::Variant;
use crate::r#enum::parse;
use crate::ir::{Region, ToDeserializeOp, ToSerializeOp, Value};
use crate::ops::algorithm::with_maybe_byte_order;
use crate::ops::{
    custom_expr, deserialize_object, error, impl_deserialize, impl_serialize, match_, member, ok, ref_, self_,
    serialize_composite, serialize_object, struct_, try_, use_,
};
use crate::r#struct::ast::Struct;
use crate::utility::deconstruct_pattern;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub ident: Ident,
    pub storage_ty: Type,
    pub generics: Generics,
    pub byte_order: Option<ByteOrder>,
    pub catch_all: Option<(Ident, bool)>,
    pub variants: Vec<Variant>,
}

impl Enum {
    fn normal_variants(&self) -> impl Iterator<Item = &Variant> {
        self.variants.iter().filter(|variant| {
            self.catch_all.as_ref().is_some_and(|(ident, _)| ident != &variant.ident) || self.catch_all.is_none()
        })
    }

    fn catch_all_variant(&self) -> Option<(&Variant, bool)> {
        self.variants
            .iter()
            .find(|variant| self.catch_all.as_ref().is_some_and(|(ident, _)| ident == &variant.ident))
            .map(|variant| (variant, self.catch_all.as_ref().expect("find ensures Some").1))
    }

    fn normal_arm_serialize(
        self_ident: &Ident,
        storage_ty: &Type,
        serializer: Value,
        variant: &Variant,
    ) -> (syn::Pat, Option<Expr>, Region) {
        let variant_ident = &variant.ident;
        let pat = if let Some(content) = &variant.content {
            deconstruct_pattern(&parse_quote!(#self_ident::#variant_ident), content.members().into_iter())
        } else {
            parse_quote!(#self_ident::#variant_ident)
        };
        let discriminant_expr = variant.discriminant.clone();
        let discriminant_cast = parse_quote!( (#discriminant_expr) as #storage_ty );
        let content = variant.content.as_ref().cloned();
        let body = Region::build(move |region: &mut Region, []| {
            if let Some(content) = content {
                let result_comp = serialize_composite(
                    region,
                    serializer,
                    Region::build(move |region, [serializer]| {
                        let discriminant = custom_expr(region, discriminant_cast);
                        let disc_ref = ref_(region, discriminant);
                        let disc_result = serialize_object(region, serializer, disc_ref, false);
                        try_(region, disc_result);
                        let result = content.serialize_members(region, serializer);
                        vec![result]
                    }),
                );
                let span_comp = try_(region, result_comp);
                let span_comp0 = member(region, span_comp, syn::Member::from(0), false);
                vec![ok(region, span_comp0)]
            } else {
                let discriminant = custom_expr(region, discriminant_cast);
                let disc_ref = ref_(region, discriminant);
                vec![serialize_object(region, serializer, disc_ref, false)]
            }
        });
        (pat, None, body)
    }

    fn catch_all_arm_serialize(
        self_ident: &Ident,
        storage_ty: &Type,
        serializer: Value,
        variant: &Variant,
        store_disc: bool,
    ) -> (syn::Pat, Option<Expr>, Region) {
        let variant_ident = &variant.ident;
        if store_disc {
            let pat = parse_quote!(#self_ident::#variant_ident(value));
            let value_expr = parse_quote!(value);
            let body = Region::build(move |region, []| {
                let discriminant = custom_expr(region, value_expr);
                let disc_ref = ref_(region, discriminant);
                vec![serialize_object(region, serializer, disc_ref, false)]
            });
            (pat, None, body)
        } else {
            let pat = parse_quote!(#self_ident::#variant_ident);
            let discriminant_expr = variant.discriminant.clone();
            let discriminant_cast = parse_quote!( (#discriminant_expr) as #storage_ty );
            let body = Region::build(move |region, []| {
                let discriminant = custom_expr(region, discriminant_cast);
                let disc_ref = ref_(region, discriminant);
                vec![serialize_object(region, serializer, disc_ref, false)]
            });
            (pat, None, body)
        }
    }

    fn catch_all_arm_deserialize(
        self_ident: &Ident,
        discriminant: Value,
        variant: &Variant,
        store_disc: bool,
    ) -> (syn::Pat, Option<Expr>, Region) {
        let variant_ident = &variant.ident;
        let pat = parse_quote!(_);
        if store_disc {
            let struct_ty = parse_quote!(#self_ident::#variant_ident);
            let body = Region::build(move |region, []| {
                let value = struct_(region, struct_ty, vec![(syn::Member::from(0), discriminant)]);
                vec![ok(region, value)]
            });
            (pat, None, body)
        } else {
            let struct_ty = parse_quote!(#self_ident::#variant_ident);
            let body = Region::build(move |region, []| {
                let value = struct_(region, struct_ty, vec![]);
                vec![ok(region, value)]
            });
            (pat, None, body)
        }
    }

    fn normal_arm_deserialize(
        self_ident: &Ident,
        variant: &Variant,
        deserializer: Value,
    ) -> (syn::Pat, Option<Expr>, Region) {
        let variant_ident = variant.ident.clone();
        let pat = parse_quote!(discriminant);
        let discriminant_expr = &variant.discriminant;
        let guard_expr = parse_quote!(discriminant == #discriminant_expr);
        let struct_ty = parse_quote!(#self_ident::#variant_ident);
        let content = variant.content.as_ref().cloned();
        let self_ident = self_ident.clone();
        let body = Region::build(move |region, []| {
            if let Some(content) = content {
                use_(region, parse_quote!(#self_ident::#variant_ident));
                vec![content.deserialize_members(region, deserializer)]
            } else {
                let value = struct_(region, struct_ty, vec![]);
                vec![ok(region, value)]
            }
        });
        (pat, Some(guard_expr), body)
    }
}

impl TryFrom<parse::Enum> for Enum {
    type Error = syn::Error;
    fn try_from(mut value: parse::Enum) -> Result<Self, Self::Error> {
        let storage_ty = value.storage_ty.unwrap_or(parse_quote!(isize));
        let mut catch_all_it = value
            .variants
            .iter_mut()
            .filter_map(|variant| variant.catch_all.take().map(|ty| (variant.ident.clone(), ty)));
        let catch_all = catch_all_it.next();
        if let Some((_ident, ty)) = &catch_all {
            if let Some((ident, _)) = catch_all_it.next() {
                return Err(syn::Error::new(ident.span(), "there may be at most one catch all variant"));
            }
            if let Some(ty) = ty {
                if ty != &storage_ty {
                    return Err(syn::Error::new(
                        ty.span(),
                        format!(
                            "catch all type ({}) differs from the enum's repr ({})",
                            ty.to_token_stream(),
                            storage_ty.to_token_stream()
                        ),
                    ));
                }
            }
        }

        let discriminants = compute_discriminants(value.variants.iter_mut().map(|variant| variant.discriminant.take()));
        let variants = std::iter::zip(value.variants.into_iter(), discriminants.into_iter())
            .map(|(variant, discriminant)| -> Result<Variant, syn::Error> {
                let content = variant.content.map(|content| Struct::try_from(content)).transpose()?;
                Ok(Variant { ident: variant.ident, discriminant, content })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            ident: value.ident,
            storage_ty,
            generics: value.generics,
            byte_order: value.byte_order,
            catch_all: catch_all.map(|(ident, ty)| (ident, ty.is_some())),
            variants,
        })
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
                    let normal_arms = self
                        .normal_variants()
                        .map(|variant| Self::normal_arm_serialize(&self.ident, &self.storage_ty, serializer, variant));
                    let catch_all_arm = self.catch_all_variant().map(|(variant, store_disc)| {
                        Self::catch_all_arm_serialize(&self.ident, &self.storage_ty, serializer, variant, store_disc)
                    });
                    let arms = normal_arms.chain(catch_all_arm);
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
                        let normal_arms = self
                            .normal_variants()
                            .map(|variant| Self::normal_arm_deserialize(&self.ident, variant, deserializer));
                        let unmatched_arm = self
                            .catch_all_variant()
                            .map(|(variant, store_disc)| {
                                Self::catch_all_arm_deserialize(&self.ident, discriminant, variant, store_disc)
                            })
                            .unwrap_or_else(|| mismatch_arm_deserialize(deserializer));
                        let arms = normal_arms.chain(std::iter::once(unmatched_arm));
                        match_(region, discriminant, arms.collect())
                    });
                vec![result]
            }),
        );
        vec![]
    }
}

fn mismatch_arm_deserialize(deserializer: Value) -> (syn::Pat, Option<Expr>, Region) {
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

impl Enum {
    pub fn is_multi_pass(&self) -> bool {
        self.variants
            .iter()
            .filter_map(|variant| variant.content.as_ref())
            .any(|content| content.is_multi_pass())
    }

    pub fn to_pack_into_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let storage_ty = &self.storage_ty;

        let exact_arms = self.normal_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            quote! { Self::#ident => { ((#discr_expr) as #storage_ty).pack_into(num_bits) } }
        });
        let catch_all_arm = self.catch_all_variant().map(|(variant, store_disc)| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            if store_disc {
                quote! { Self::#ident(value) => { value.pack_into(num_bits) } }
            } else {
                quote! { Self::#ident => { ((#discr_expr) as #storage_ty).pack_into(num_bits) } }
            }
        });
        let arms = exact_arms.chain(catch_all_arm);

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

        let exact_arms = self.normal_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            quote! { n if n == (#discr_expr) as u8 => { ::core::result::Result::Ok(Self::#ident) } }
        });
        let catch_all_arm = self
            .catch_all_variant()
            .map(|(variant, store_disc)| {
                let ident = &variant.ident;
                if store_disc {
                    quote! { n => { ::core::result::Result::Ok(Self::#ident(n)) } }
                } else {
                    quote! { _ => { ::core::result::Result::Ok(Self::#ident) } }
                }
            })
            .unwrap_or_else(|| {
                quote! { _ => { Err(value) } }
            });
        let arms = exact_arms.chain(std::iter::once(catch_all_arm));

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
            catch_all: None,
            variants: vec![
                Variant { ident: parse_quote!(A), discriminant: parse_quote!(0), content: None },
                Variant { ident: parse_quote!(B), discriminant: parse_quote!(1), content: None },
            ],
        }
    }

    fn create_catch_all_empty() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            catch_all: Some((parse_quote!(CatchAll), false)),
            variants: vec![
                Variant { ident: parse_quote!(A), discriminant: parse_quote!(0), content: None },
                Variant { ident: parse_quote!(CatchAll), discriminant: parse_quote!(1), content: None },
            ],
        }
    }

    fn create_catch_all_tuple() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            catch_all: Some((parse_quote!(CatchAll), true)),
            variants: vec![
                Variant { ident: parse_quote!(A), discriminant: parse_quote!(0), content: None },
                Variant { ident: parse_quote!(CatchAll), discriminant: parse_quote!(1), content: None },
            ],
        }
    }

    fn create_fielded() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            catch_all: Some((parse_quote!(CatchAll), true)),
            variants: vec![
                Variant {
                    ident: parse_quote!(A),
                    discriminant: parse_quote!(0),
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
                    discriminant if discriminant == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discriminant if discriminant == 1 => {
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
                    Test :: CatchAll (value) => {
                        %disc_ca = custom_expr [value]
                        %disc_ca_ref = ref %disc_ca
                        %result_ca = serialize_object [false] %serializer, %disc_ca_ref
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
                    discriminant if discriminant == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    _ => {
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
                    discriminant if discriminant == 0 => {
                        %result_a = struct [Test::A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    _ => {
                        %result_ca = struct [Test::CatchAll, 0] %discriminant
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
                    discriminant if discriminant == 0 => {
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
                    discriminant if discriminant == 1 => {
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
