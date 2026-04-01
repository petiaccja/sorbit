use syn::{Expr, Ident, Member};

use crate::r#struct::ast::Struct;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Expr,
    pub catch_all: CatchAll,
    pub content: Option<Struct>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatchAll {
    None,
    Blanket,
    Discriminant(Member),
}
