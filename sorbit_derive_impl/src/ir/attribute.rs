use std::fmt::Display;

use quote::ToTokens;

use crate::attribute::{BitNumbering, ByteOrder};

pub trait Attribute {
    fn display(&self) -> String;
}

impl Attribute for syn::Path {
    fn display(&self) -> String {
        self.to_token_stream().to_string().replace(" :: ", "::").replace(":: ", "::")
    }
}

impl Attribute for syn::Type {
    fn display(&self) -> String {
        self.to_token_stream().to_string().replace(" :: ", "::").replace(":: ", "::")
    }
}

impl Attribute for syn::Generics {
    fn display(&self) -> String {
        self.to_token_stream().to_string().replace(" :: ", "::").replace(":: ", "::")
    }
}

impl Attribute for syn::Expr {
    fn display(&self) -> String {
        self.to_token_stream().to_string()
    }
}

impl Attribute for Vec<(syn::Member, syn::Ident)> {
    fn display(&self) -> String {
        let members: Vec<_> =
            self.iter().map(|(member, ident)| format!("{}: {}", member.display(), ident.display())).collect();
        members.join(", ")
    }
}

impl<T: Display> Attribute for std::ops::Range<T> {
    fn display(&self) -> String {
        format!("{}..{}", self.start, self.end)
    }
}

impl Attribute for syn::Member {
    fn display(&self) -> String {
        match self {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        }
    }
}

macro_rules! impl_attribute_for_display {
    ($type:ty) => {
        impl Attribute for $type {
            fn display(&self) -> String {
                self.to_string()
            }
        }
    };
}

impl_attribute_for_display!(bool);
impl_attribute_for_display!(i8);
impl_attribute_for_display!(i16);
impl_attribute_for_display!(i32);
impl_attribute_for_display!(i64);
impl_attribute_for_display!(u8);
impl_attribute_for_display!(u16);
impl_attribute_for_display!(u32);
impl_attribute_for_display!(u64);
impl_attribute_for_display!(BitNumbering);
impl_attribute_for_display!(ByteOrder);
impl_attribute_for_display!(String);
impl_attribute_for_display!(syn::Ident);
