use syn::{Expr, Ident};

use crate::r#struct::ast::Struct;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Expr,
    pub content: Option<Struct>,
}
