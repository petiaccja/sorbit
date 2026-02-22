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
