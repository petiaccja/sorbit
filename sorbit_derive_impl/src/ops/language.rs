use crate::{
    ir::{Attribute, Operation, Region, Value, op},
    utility::deconstruct_pattern,
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, Member, Path, Type};

//------------------------------------------------------------------------------
// Self
//------------------------------------------------------------------------------

op!(
    name: "self",
    builder: self_,
    op: SelfOp,
    inputs: {},
    outputs: {self_value},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for SelfOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! { self })
    }
}

//------------------------------------------------------------------------------
// Ref
//------------------------------------------------------------------------------

op!(
    name: "ref",
    builder: ref_,
    op: RefOp,
    inputs: {value},
    outputs: {ref_value},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for RefOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        tokens.extend(quote! { &#value })
    }
}

//------------------------------------------------------------------------------
// Use
//------------------------------------------------------------------------------

op!(
    name: "use",
    builder: use_,
    op: UseOp,
    inputs: {},
    outputs: {},
    attributes: {path: Path},
    regions: {},
    terminator: false
);

impl ToTokens for UseOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = &self.path;
        tokens.extend(quote! { use #path })
    }
}

//------------------------------------------------------------------------------
// Destructure
//------------------------------------------------------------------------------

op!(
    name: "destructure",
    builder: destructure,
    op: DestructureOp,
    inputs: {structured},
    outputs: {},
    attributes: {ty: Type, bindings: Vec<(Member, Ident)>},
    regions: {},
    terminator: false
);

impl ToTokens for DestructureOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let structured = self.structured;
        let members = self.bindings.iter().map(|(member, _)| member);
        let pat = deconstruct_pattern(&self.ty, members.into_iter());
        tokens.extend(quote! { let #pat = #structured })
    }
}

//------------------------------------------------------------------------------
// SymOp
//------------------------------------------------------------------------------

op!(
    name: "sym",
    builder: sym,
    op: SymOp,
    inputs: {value},
    outputs: {},
    attributes: {sym: Ident},
    regions: {},
    terminator: false
);

impl ToTokens for SymOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        let sym = &self.sym;
        tokens.extend(quote! { #[allow(unused)] let #sym = &#value })
    }
}

//------------------------------------------------------------------------------
// SymrefOp
//------------------------------------------------------------------------------

op!(
    name: "symref",
    builder: symref,
    op: SymrefOp,
    inputs: {},
    outputs: {value},
    attributes: {sym: Ident},
    regions: {},
    terminator: false
);

impl ToTokens for SymrefOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let sym = &self.sym;
        tokens.extend(quote! { #sym })
    }
}

//------------------------------------------------------------------------------
// Yield
//------------------------------------------------------------------------------

pub struct YieldOp {
    values: Vec<Value>,
}

pub fn yield_(region: &mut Region, values: Vec<Value>) {
    region.append(YieldOp { values });
}

impl Operation for YieldOp {
    fn name(&self) -> &str {
        "yield"
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
}

impl ToTokens for YieldOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let values = &self.values;
        tokens.extend(match values.len() {
            0 => quote! { () },
            1 => quote! { #(#values)* },
            _ => quote! { (#(#values),*) },
        });
    }
}

//------------------------------------------------------------------------------
// Member
//------------------------------------------------------------------------------

op!(
    name: "member",
    builder: member,
    op: MemberOp,
    inputs: {value},
    outputs: {member_value},
    attributes: {member: syn::Member, reference: bool},
    regions: {},
    terminator: false
);

impl ToTokens for MemberOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        let member = &self.member;
        match self.reference {
            false => tokens.extend(quote! { #value.#member }),
            true => tokens.extend(quote! { &#value.#member }),
        }
    }
}

//------------------------------------------------------------------------------
// Try
//------------------------------------------------------------------------------

op!(
    name: "try",
    builder: try_,
    op: TryOp,
    inputs: {value},
    outputs: {result},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for TryOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        tokens.extend(quote! { #value? })
    }
}

//------------------------------------------------------------------------------
// Match
//------------------------------------------------------------------------------

pub struct MatchOp {
    expr: Value,
    arms: Vec<(syn::Pat, Option<syn::Expr>, Region)>,
    result: Value,
}

pub fn match_<'a>(region: &mut Region, expr: Value, arms: Vec<(syn::Pat, Option<syn::Expr>, Region)>) -> Value {
    region.append(MatchOp { expr, arms, result: Value::new() })[0]
}

impl Operation for MatchOp {
    fn name(&self) -> &str {
        "match"
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.expr]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.result]
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

impl ToTokens for MatchOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = self.expr;
        let arms = self.arms.iter().map(|(pattern, guard, region)| match guard {
            Some(guard) => quote! { #pattern if #guard => #region },
            None => quote! { #pattern => #region },
        });
        tokens.extend(quote! {
            match #expr {
                #(#arms)*
            }
        });
    }
}

//------------------------------------------------------------------------------
// Expr
//------------------------------------------------------------------------------

// Introducing [`syn`] expressions into the SSA IR.
//
// This operation is necessary when it's impossible or impractical to convert
// an expression to the SSA IR. This is the case, for example, for enum
// discriminants, which can be arbitrary expressions like calling a const
// function.

op!(
    name: "custom_expr",
    builder: custom_expr,
    op: CustomExprOp,
    inputs: {},
    outputs: {expr_value},
    attributes: {expr: syn::Expr},
    regions: {},
    terminator: false
);

impl ToTokens for CustomExprOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! { #expr })
    }
}

//------------------------------------------------------------------------------
// Ok
//------------------------------------------------------------------------------

op!(
    name: "ok",
    builder: ok,
    op: OkOp,
    inputs: {value},
    outputs: {ok_value},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for OkOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        tokens.extend(quote! { ::core::result::Result::Ok(#value) })
    }
}

//------------------------------------------------------------------------------
// Tuple
//------------------------------------------------------------------------------

pub struct TupleOp {
    members: Vec<Value>,
    result: Value,
}

pub fn tuple(region: &mut Region, members: Vec<Value>) -> Value {
    region.append(TupleOp { members, result: Value::new() })[0]
}

impl Operation for TupleOp {
    fn name(&self) -> &str {
        "tuple"
    }

    fn inputs(&self) -> Vec<Value> {
        self.members.clone()
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.result]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }
}

impl ToTokens for TupleOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let members = &self.members;
        tokens.extend(quote! { (#(#members,)*) });
    }
}

//------------------------------------------------------------------------------
// Struct
//------------------------------------------------------------------------------

pub struct StructOp {
    struct_ty: syn::Type,
    members: Vec<(syn::Member, Value)>,
    result: Value,
}

pub fn struct_(region: &mut Region, struct_ty: syn::Type, members: Vec<(syn::Member, Value)>) -> Value {
    region.append(StructOp { struct_ty, members, result: Value::new() })[0]
}

impl Operation for StructOp {
    fn name(&self) -> &str {
        "struct"
    }
    fn inputs(&self) -> Vec<Value> {
        self.members.iter().map(|(_, value)| *value).collect()
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.result]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        let mut attrs = vec![self.struct_ty.display()];
        attrs.extend(self.members.iter().map(|(member, _)| member.display()));
        attrs
    }
}

impl ToTokens for StructOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty = &self.struct_ty;
        let members = self.members.iter().map(|(member, _)| member);
        let values = self.members.iter().map(|(_, value)| value);
        tokens.extend(quote! { #ty{ #(#members: #values,)* } });
    }
}
