use std::ops::Range;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::constants::{
    BIT_FIELD_TYPE, DESERIALIZE_TRAIT, DESERIALIZER_OBJECT, DESERIALIZER_TRAIT, DESERIALIZER_TYPE, ERROR_TRAIT,
    SERIALIZE_TRAIT, SERIALIZER_OBJECT, SERIALIZER_OUTPUT_TRAIT, SERIALIZER_TRAIT, SERIALIZER_TYPE,
};
use super::{
    AndThen, Block, DeserializeComposite, DeserializeNothing, DeserializeObject, Direction, Enclose, Expr,
    ImplDeserialize, ImplSerialize, IntoBitField, Layout, Let, MakeStruct, MakeTuple, NewBitField, Ok, PackBitField,
    PackObject, SerializeComposite, SerializeNothing, SerializeObject, Statement, SymRef, Try, UnpackObject,
};

//------------------------------------------------------------------------------
// Trait implementation nodes.
//------------------------------------------------------------------------------

impl ToTokens for ImplSerialize {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let body = &self.body;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            #[automatically_derived]
            impl #impl_generics #SERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn serialize<#SERIALIZER_TYPE: #SERIALIZER_TRAIT>(
                    &self,
                    #SERIALIZER_OBJECT: &mut #SERIALIZER_TYPE
                ) -> ::core::result::Result<
                        <#SERIALIZER_TYPE as #SERIALIZER_OUTPUT_TRAIT>::Success,
                        <#SERIALIZER_TYPE as #SERIALIZER_OUTPUT_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        });
    }
}

impl ToTokens for ImplDeserialize {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let body = &self.body;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            #[automatically_derived]
            impl #impl_generics #DESERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn deserialize<#DESERIALIZER_TYPE: #DESERIALIZER_TRAIT>(
                    #DESERIALIZER_OBJECT: &mut #DESERIALIZER_TYPE
                ) -> ::core::result::Result<
                        Self,
                        <#DESERIALIZER_TYPE as #DESERIALIZER_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        });
    }
}

//------------------------------------------------------------------------------
// Polymorphic expression and statement nodes.
//------------------------------------------------------------------------------

impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Expr::Try(value) => value.to_tokens(tokens),
            Expr::MakeTuple(value) => value.to_tokens(tokens),
            Expr::MakeStruct(value) => value.to_tokens(tokens),
            Expr::AndThen(value) => value.to_tokens(tokens),
            Expr::Ok(value) => value.to_tokens(tokens),
            Expr::Block(value) => value.to_tokens(tokens),
            Expr::Symref(value) => value.to_tokens(tokens),
            Expr::Enclose(value) => value.to_tokens(tokens),
            Expr::Layout(value) => value.to_tokens(tokens),
            Expr::SerializeNothing(value) => value.to_tokens(tokens),
            Expr::SerializeObject(value) => value.to_tokens(tokens),
            Expr::SerializeComposite(value) => value.to_tokens(tokens),
            Expr::DeserializeNothing(value) => value.to_tokens(tokens),
            Expr::DeserializeObject(value) => value.to_tokens(tokens),
            Expr::DeserializeComposite(value) => value.to_tokens(tokens),
            Expr::NewBitField(value) => value.to_tokens(tokens),
            Expr::IntoBitField(value) => value.to_tokens(tokens),
            Expr::PackObject(value) => value.to_tokens(tokens),
            Expr::PackBitField(value) => value.to_tokens(tokens),
            Expr::UnpackObject(value) => value.to_tokens(tokens),
        }
    }
}

impl ToTokens for Statement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Statement::Let(value) => value.to_tokens(tokens),
        }
    }
}

//------------------------------------------------------------------------------
// Language expression nodes.
//------------------------------------------------------------------------------

impl ToTokens for Try {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! { #expr? });
    }
}

impl ToTokens for MakeTuple {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let elements = self.elements.iter().map(|element| element.to_token_stream());
        tokens.extend(quote! { (#(#elements),*) });
    }
}

impl ToTokens for MakeStruct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let members = self.members.iter().map(|(member, _)| member.to_token_stream());
        let values = self.members.iter().map(|(_, value)| value.to_token_stream());
        tokens.extend(quote! { #name { #(#members : #values),* } });
    }
}

impl ToTokens for AndThen {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { result, value, expr } = self;
        match value {
            Some(ident) => tokens.extend(quote! { ( #result ).and_then(|#ident| #expr) }),
            None => tokens.extend(quote! { ( #result ).and_then(|_| #expr) }),
        }
    }
}

impl ToTokens for Ok {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! { Ok(#expr) });
    }
}

impl ToTokens for Block {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let statements = self.statements.iter().map(|statement| statement.to_token_stream());
        let result = &self.result;
        tokens.extend(quote! {
            {
                #(#statements)*
                #result
            }
        });
    }
}

impl ToTokens for SymRef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        tokens.extend(quote! { #ident });
    }
}

//------------------------------------------------------------------------------
// Language statement nodes.
//------------------------------------------------------------------------------

impl ToTokens for Let {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        match &self.ident {
            Some(ident) => tokens.extend(quote! { let #ident = #expr; }),
            None => tokens.extend(quote! { let _ = #expr; }),
        }
    }
}

//------------------------------------------------------------------------------
// Serialization expression nodes.
//------------------------------------------------------------------------------

impl ToTokens for Enclose {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        let item = &self.item;
        tokens.extend(quote! {
            #expr.map_err(|err| #ERROR_TRAIT::enclose(err, #item))
        });
    }
}

impl ToTokens for Layout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        let (ser_trait, ser_obj, ser_composite_fn) = match self.direction {
            Direction::Serialize => {
                (quote! { #SERIALIZER_TRAIT }, quote! { #SERIALIZER_OBJECT }, quote! { serialize_composite })
            }
            Direction::Deserialize => {
                (quote! { #DESERIALIZER_TRAIT }, quote! { #DESERIALIZER_OBJECT }, quote! { deserialize_composite })
            }
        };

        let local = quote! { #expr };
        let local = match self.len {
            Some(len) => quote! { #local.and_then(|r| #ser_trait::pad(#ser_obj, #len).map(|_| r)) },
            None => local,
        };
        let local = match self.round {
            Some(round) => quote! { #local.and_then(|r| #ser_trait::align(#ser_obj, #round).map(|_| r)) },
            None => local,
        };
        let local = match self.len.is_some() || self.round.is_some() {
            true => quote! {
                #ser_trait::#ser_composite_fn(#ser_obj, |#ser_obj| {
                    #local
                }).map(|(_, r)| r)
            },
            false => local,
        };

        let global = match self.align {
            Some(align) => quote! { #ser_trait::align(#ser_obj, #align).and_then(#local) },
            None => local,
        };
        let global = match self.offset {
            Some(offset) => quote! { #ser_trait::pad(#ser_obj, #offset).and_then(#global) },
            None => global,
        };

        tokens.extend(global);
    }
}

impl ToTokens for SerializeNothing {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! { #SERIALIZER_TRAIT::serialize_nothing(#SERIALIZER_OBJECT)});
    }
}

impl ToTokens for SerializeObject {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let object = &self.object;
        tokens.extend(quote! { #SERIALIZE_TRAIT::serialize(#object, #SERIALIZER_OBJECT)});
    }
}

impl ToTokens for SerializeComposite {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! {
            #SERIALIZER_TRAIT::serialize_composite(#SERIALIZER_OBJECT, |#SERIALIZER_OBJECT| {
                #expr
            })
        });
    }
}

impl ToTokens for DeserializeNothing {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! { #DESERIALIZER_TRAIT::deserialize_nothing(#DESERIALIZER_OBJECT)});
    }
}

impl ToTokens for DeserializeObject {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty = &self.ty;
        tokens.extend(quote! { <#ty as #DESERIALIZE_TRAIT>::deserialize(#DESERIALIZER_OBJECT)});
    }
}

impl ToTokens for DeserializeComposite {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! {
            #DESERIALIZER_TRAIT::deserialize_composite(#DESERIALIZER_OBJECT, |#DESERIALIZER_OBJECT| {
                #expr
            })
        });
    }
}

impl ToTokens for NewBitField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty = &self.ty;
        tokens.extend(quote! { #BIT_FIELD_TYPE::new::<#ty>() });
    }
}

impl ToTokens for IntoBitField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let packed = &self.packed;
        tokens.extend(quote! { #BIT_FIELD_TYPE::from_bits(#packed) });
    }
}

impl ToTokens for PackObject {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { bit_field, object, bit_range: Range { start, end } } = self;
        tokens.extend(quote! {
            #bit_field.pack(#object, #start..#end)
                      .map_err(|err| ::sorbit::codegen::bit_error_to_error_se(#SERIALIZER_OBJECT, err))
        });
    }
}

impl ToTokens for PackBitField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { bit_field, packed_ty, members } = self;
        tokens.extend(quote! {
            {
                let mut #bit_field = #BIT_FIELD_TYPE::<#packed_ty>::new();
                let results = [
                    #(#members,)*
                ];
                results.into_iter().fold(Ok(()), |acc, result| acc.and(result)).map(|_| bit_field)
            }
        });
    }
}

impl ToTokens for UnpackObject {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { bit_field, ty, bit_range: Range { start, end } } = self;
        tokens.extend(quote! {
            #bit_field.unpack::<#ty, _, _>(#start..#end)
                      .map_err(|err| ::sorbit::codegen::bit_error_to_error_de(#DESERIALIZER_OBJECT, err))
        });
    }
}
