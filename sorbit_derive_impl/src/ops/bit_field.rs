use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::attribute::BitNumbering;
use crate::ir::dag::{Id, Operation, Region, Value};
use crate::ops::constants::BIT_FIELD_TYPE;

//------------------------------------------------------------------------------
// Empty bit field
//------------------------------------------------------------------------------

struct EmptyBitFieldOp {
    id: Id,
    packed_ty: syn::Type,
}

pub fn empty_bit_field(region: &mut Region, packed_ty: syn::Type) -> Value {
    region.push(EmptyBitFieldOp { id: Id::new(), packed_ty })[0]
}

impl Operation for EmptyBitFieldOp {
    fn name(&self) -> &str {
        "empty_bit_field"
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
        vec![self.packed_ty.to_token_stream().to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let packed_ty = &self.packed_ty;
        quote! { #BIT_FIELD_TYPE::<#packed_ty>::new() }
    }
}

//------------------------------------------------------------------------------
// Into bit field
//------------------------------------------------------------------------------

struct IntoBitFieldOp {
    id: Id,
    packed: Value,
}

pub fn into_bit_field(region: &mut Region, packed: Value) -> Value {
    region.push(IntoBitFieldOp { id: Id::new(), packed })[0]
}

impl Operation for IntoBitFieldOp {
    fn name(&self) -> &str {
        "into_bit_field"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.packed]
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
        let packed = &self.packed;
        quote! { #BIT_FIELD_TYPE::from_bits(#packed) }
    }
}

//------------------------------------------------------------------------------
// Into raw bits
//------------------------------------------------------------------------------

struct IntoRawBitsOp {
    id: Id,
    bit_field: Value,
}

pub fn into_raw_bits(region: &mut Region, bit_field: Value) -> Value {
    region.push(IntoRawBitsOp { id: Id::new(), bit_field })[0]
}

impl Operation for IntoRawBitsOp {
    fn name(&self) -> &str {
        "into_raw_bits"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.bit_field]
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
        let bit_field = &self.bit_field;
        quote! { #bit_field.into_bits() }
    }
}

//------------------------------------------------------------------------------
// Pack bit field
//------------------------------------------------------------------------------

struct PackBitFieldOp {
    id: Id,
    value: Value,
    bit_field: Value,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
}

pub fn pack_bit_field(
    region: &mut Region,
    value: Value,
    bit_field: Value,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
) -> Value {
    region.push(PackBitFieldOp { id: Id::new(), value, bit_field, bits, bit_numbering })[0]
}

impl Operation for PackBitFieldOp {
    fn name(&self) -> &str {
        "pack_bit_field"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.value, self.bit_field]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![
            format!("{}..{}", self.bits.start, self.bits.end),
            format!("{:?}", self.bit_numbering),
        ]
    }

    fn to_token_stream(&self) -> TokenStream {
        let value = &self.value;
        let bit_field = &self.bit_field;
        let start = self.bits.start;
        let end = self.bits.end;
        let bit_range = bit_range_to_token_stream(quote! {bit_field}, start, end, self.bit_numbering);
        quote! {
            {
                let mut bit_field = #bit_field;
                bit_field.pack(&#value, #bit_range)
                          .map_err(|err| err.into())
                          .map(|_| bit_field)
            }
        }
    }
}

fn bit_range_to_token_stream(bit_field: impl ToTokens, start: u8, end: u8, bit_numbering: BitNumbering) -> TokenStream {
    let bit_range = match bit_numbering {
        BitNumbering::MSB0 => {
            quote! { (#bit_field.bit_size_of() as u8 - #end)..(#bit_field.bit_size_of() as u8 - #start) }
        }
        BitNumbering::LSB0 => quote! { #start..#end },
    };
    bit_range
}

//------------------------------------------------------------------------------
// Unpack bit field
//------------------------------------------------------------------------------

struct UnpackBitFieldOp {
    id: Id,
    bit_field: Value,
    ty: syn::Type,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
}

pub fn unpack_bit_field(
    region: &mut Region,
    bit_field: Value,
    ty: syn::Type,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
) -> Value {
    region.push(UnpackBitFieldOp { id: Id::new(), bit_field, ty, bits, bit_numbering })[0]
}

impl Operation for UnpackBitFieldOp {
    fn name(&self) -> &str {
        "unpack_bit_field"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.bit_field]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![
            self.ty.to_token_stream().to_string(),
            format!("{}..{}", self.bits.start, self.bits.end),
            format!("{:?}", self.bit_numbering),
        ]
    }

    fn to_token_stream(&self) -> TokenStream {
        let bit_field = &self.bit_field;
        let ty = &self.ty;
        let start = self.bits.start;
        let end = self.bits.end;
        let bit_range = bit_range_to_token_stream(bit_field, start, end, self.bit_numbering);
        quote! { #bit_field.unpack::<#ty, _, _>(#bit_range).map_err(|err| err.into()) }
    }
}
