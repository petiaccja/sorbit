use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::attribute::BitNumbering;
use crate::ir::op;
use crate::ops::constants::BIT_FIELD_TYPE;

op!(
    name: "empty_bit_field",
    builder: empty_bit_field,
    op: EmptyBitFieldOp,
    inputs: {},
    outputs: {result},
    attributes: {packed_ty: syn::Type},
    regions: {},
    terminator: false
);

impl ToTokens for EmptyBitFieldOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let packed_ty = &self.packed_ty;
        tokens.extend(quote! { #BIT_FIELD_TYPE::<#packed_ty>::new() })
    }
}

op!(
    name: "pack_bit_field",
    builder: pack_bit_field,
    op: PackBitFieldOp,
    inputs: {value, bit_field},
    outputs: {packed_bit_field},
    attributes: {bits: std::ops::Range<u8>, bit_numbering: BitNumbering},
    regions: {},
    terminator: false
);

impl ToTokens for PackBitFieldOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        let bit_field = &self.bit_field;
        let start = self.bits.start;
        let end = self.bits.end;
        let bit_range = bit_range_to_token_stream(quote! {bit_field}, start, end, self.bit_numbering);
        tokens.extend(quote! {
            {
                let mut bit_field = #bit_field;
                bit_field.pack(&#value, #bit_range)
                          .map_err(|err| err.into())
                          .map(|_| bit_field)
            }
        })
    }
}

op!(
    name: "unpack_bit_field",
    builder: unpack_bit_field,
    op: UnpackBitFieldOp,
    inputs: {bit_field},
    outputs: {value},
    attributes: {ty: syn::Type, bits: std::ops::Range<u8>, bit_numbering: BitNumbering},
    regions: {},
    terminator: false
);

impl ToTokens for UnpackBitFieldOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let bit_field = &self.bit_field;
        let ty = &self.ty;
        let start = self.bits.start;
        let end = self.bits.end;
        let bit_range = bit_range_to_token_stream(bit_field, start, end, self.bit_numbering);
        tokens.extend(quote! { #bit_field.unpack::<#ty, _, _>(#bit_range).map_err(|err| err.into()) })
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
