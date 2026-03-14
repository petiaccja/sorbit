use std::sync::atomic::{AtomicUsize, Ordering};

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);

impl Id {
    pub fn new() -> Self {
        Self(next_id())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value {
    pub id: Id,
}

impl Value {
    pub fn new() -> Self {
        Self { id: Id::new() }
    }

    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("v{}", self.id.0)
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_ident().to_tokens(tokens);
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.id.0)
    }
}
