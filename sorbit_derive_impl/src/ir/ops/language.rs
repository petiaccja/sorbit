use crate::ir::dag::{Id, Operation, Region, Value};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

//------------------------------------------------------------------------------
// Self
//------------------------------------------------------------------------------

pub struct SelfOp {
    id: Id,
}

pub fn self_(region: &mut Region) -> Value {
    region.push(SelfOp { id: Id::new() })[0]
}

impl Operation for SelfOp {
    fn name(&self) -> &str {
        "self"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        quote! { self }
    }
}

//------------------------------------------------------------------------------
// Ref
//------------------------------------------------------------------------------

pub struct RefOp {
    id: Id,
    value: Value,
}

pub fn ref_(region: &mut Region, value: Value) -> Value {
    region.push(RefOp { id: Id::new(), value })[0]
}

impl Operation for RefOp {
    fn name(&self) -> &str {
        "ref"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.value]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let value = &self.value;
        quote! { &#value }
    }
}

//------------------------------------------------------------------------------
// Yield
//------------------------------------------------------------------------------

pub struct YieldOp {
    id: Id,
    values: Vec<Value>,
}

pub fn yield_(region: &mut Region, values: Vec<Value>) {
    region.push(YieldOp { id: Id::new(), values });
}

impl Operation for YieldOp {
    fn name(&self) -> &str {
        "yield"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        self.values.clone()
    }

    fn outputs(&self) -> Vec<Value> {
        vec![]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn is_terminator(&self) -> bool {
        true
    }

    fn to_token_stream(&self) -> TokenStream {
        let values = &self.values;
        match values.len() {
            0 => quote! { () },
            1 => quote! { #(#values)* },
            _ => quote! { (#(#values),*) },
        }
    }
}

//------------------------------------------------------------------------------
// Member
//------------------------------------------------------------------------------

pub struct MemberOp {
    id: Id,
    value: Value,
    member: syn::Member,
    reference: bool,
}

pub fn member(region: &mut Region, value: Value, member: syn::Member, reference: bool) -> Value {
    region.push(MemberOp { id: Id::new(), value, member, reference })[0]
}

impl Operation for MemberOp {
    fn name(&self) -> &str {
        "member"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.value]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![
            self.member.to_token_stream().to_string(),
            match self.reference {
                true => "ref",
                false => "val",
            }
            .to_owned(),
        ]
    }

    fn to_token_stream(&self) -> TokenStream {
        let value = &self.value;
        let member = &self.member;
        match self.reference {
            false => quote! { #value.#member },
            true => quote! { &#value.#member },
        }
    }
}

//------------------------------------------------------------------------------
// Try
//------------------------------------------------------------------------------

pub struct TryOp {
    id: Id,
    value: Value,
}

pub fn try_(region: &mut Region, value: Value) -> Value {
    region.push(TryOp { id: Id::new(), value })[0]
}

impl Operation for TryOp {
    fn name(&self) -> &str {
        "try"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.value]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let value = &self.value;
        quote! { #value? }
    }
}

//------------------------------------------------------------------------------
// Match
//------------------------------------------------------------------------------

pub struct MatchOp {
    id: Id,
    expr: Value,
    arms: Vec<(syn::Pat, Option<syn::Expr>, Region)>,
}

pub fn match_<'a>(
    region: &mut Region,
    expr: Value,
    arms: impl Iterator<Item = (syn::Pat, Option<syn::Expr>, Box<dyn FnOnce(&mut Region) -> Value>)>,
) -> Value {
    let arms = arms
        .map(|(pattern, guard, arm_fn)| {
            let mut arm_region = Region::new(0);
            let result = arm_fn(&mut arm_region);
            let _ = yield_(&mut arm_region, vec![result]);
            (pattern, guard, arm_region)
        })
        .collect();
    region.push(MatchOp { id: Id::new(), expr, arms })[0]
}

impl Operation for MatchOp {
    fn name(&self) -> &str {
        "match"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.expr]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        self.arms.iter().map(|arm| &arm.2).collect()
    }

    fn attributes(&self) -> Vec<String> {
        self.arms
            .iter()
            .map(|(pat, guard, _region)| {
                let pat_s = pat.to_token_stream().to_string();
                let guard_s = guard.as_ref().map(|guard| guard.to_token_stream().to_string());
                match guard_s {
                    Some(guard) => format!("{pat_s} if {guard}"),
                    None => pat_s,
                }
            })
            .collect()
    }

    fn to_token_stream(&self) -> TokenStream {
        let expr = self.expr;
        let arms = self.arms.iter().map(|(pattern, guard, region)| match guard {
            Some(guard) => quote! { #pattern if #guard => #region },
            None => quote! { #pattern => #region },
        });
        quote! {
            match #expr {
                #(#arms)*
            }
        }
    }

    fn to_string(&self, alternate: bool) -> String {
        let arms = self
            .arms
            .iter()
            .map(|(pat, guard, region)| {
                let pat_s = pat.to_token_stream().to_string();
                let guard_s = guard.as_ref().map(|guard| guard.to_token_stream().to_string());
                let region_s = if alternate { format!("{region:#}") } else { format!("{region}") };
                match guard_s {
                    Some(guard) => format!("{pat_s} if {guard} => {region_s}"),
                    None => format!("{pat_s} => {region_s}"),
                }
            })
            .collect::<Vec<_>>();
        let line_sep = if alternate { "\n" } else { " " };
        let arms_s = arms.join(line_sep);
        let arms_s = if alternate { textwrap::indent(&arms_s, "    ") } else { arms_s };
        let expr = &self.expr;
        let result = self.outputs()[0];
        format!("{result} = match {expr} {{{line_sep}{arms_s}{line_sep}}}")
    }
}

//------------------------------------------------------------------------------
// Expr
//------------------------------------------------------------------------------

/// Introducing [`syn`] expressions into the SSA IR.
///
/// This operation is necessary when it's impossible or impractical to convert
/// an expression to the SSA IR. This is the case, for example, for enum
/// discriminants, which can be arbitrary expressions like calling a const
/// function.
pub struct CustomExprOp {
    id: Id,
    expr: syn::Expr,
}

pub fn custom_expr(region: &mut Region, expr: syn::Expr) -> Value {
    region.push(CustomExprOp { id: Id::new(), expr })[0]
}

impl Operation for CustomExprOp {
    fn name(&self) -> &str {
        "custom_expr"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.expr.to_token_stream().to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let expr = &self.expr;
        quote! { #expr }
    }
}

//------------------------------------------------------------------------------
// Ok
//------------------------------------------------------------------------------

pub struct OkOp {
    id: Id,
    value: Value,
}

pub fn ok(region: &mut Region, value: Value) -> Value {
    region.push(OkOp { id: Id::new(), value })[0]
}

impl Operation for OkOp {
    fn name(&self) -> &str {
        "ok"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.value]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let value = &self.value;
        quote! { ::core::result::Result::Ok(#value) }
    }
}

//------------------------------------------------------------------------------
// Tuple
//------------------------------------------------------------------------------

pub struct TupleOp {
    id: Id,
    members: Vec<Value>,
}

pub fn tuple(region: &mut Region, members: Vec<Value>) -> Value {
    region.push(TupleOp { id: Id::new(), members })[0]
}

impl Operation for TupleOp {
    fn name(&self) -> &str {
        "tuple"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        self.members.clone()
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let members = &self.members;
        quote! { (#(#members,)*) }
    }
}

//------------------------------------------------------------------------------
// Struct
//------------------------------------------------------------------------------

pub struct StructOp {
    id: Id,
    ty: syn::Type,
    members: Vec<(syn::Member, Value)>,
}

pub fn struct_(region: &mut Region, ty: syn::Type, members: Vec<(syn::Member, Value)>) -> Value {
    region.push(StructOp { id: Id::new(), ty, members })[0]
}

impl Operation for StructOp {
    fn name(&self) -> &str {
        "struct"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        self.members.iter().map(|(_, value)| *value).collect()
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        let mut attrs = vec![self.ty.to_token_stream().to_string()];
        attrs.extend(self.members.iter().map(|(member, _)| match member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        }));
        attrs
    }

    fn to_token_stream(&self) -> TokenStream {
        let ty = &self.ty;
        let members = self.members.iter().map(|(member, _)| member);
        let values = self.members.iter().map(|(_, value)| value);
        quote! { #ty{ #(#members: #values,)* } }
    }
}
