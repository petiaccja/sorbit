use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;

use crate::ir::Operation;
use crate::ir::Value;
use crate::ops::yield_;

pub struct Region {
    #[allow(unused)]
    arguments: Vec<Value>,
    operations: Vec<Box<dyn Operation>>,
}

impl Region {
    pub fn new(num_arguments: usize) -> Self {
        let arguments: Vec<_> = (0..num_arguments).map(|_| Value::new()).collect();
        Self { arguments, operations: Vec::new() }
    }

    pub fn build<const NUM_ARGUMENTS: usize>(
        builder: impl FnOnce(&mut Region, [Value; NUM_ARGUMENTS]) -> Vec<Value>,
    ) -> Self {
        let arguments = std::array::from_fn(|_| Value::new());
        let mut region = Self { arguments: arguments.into(), operations: Vec::new() };
        let results = builder(&mut region, arguments);
        yield_(&mut region, results);
        region
    }

    pub fn arguments(&self) -> &[Value] {
        &self.arguments
    }

    pub fn append(&mut self, operation: impl Operation + 'static) -> Vec<Value> {
        if self.operations.last().is_some_and(|last| last.is_terminator()) {
            panic!("region has already been terminated")
        }
        let outputs = operation.outputs();
        self.operations.push(Box::new(operation) as Box<dyn Operation>);
        outputs
    }

    pub fn to_token_stream_formatted(&self, with_braces: bool) -> TokenStream {
        let ops = self.operations.iter().map(|op| {
            let outputs = op.outputs();
            if op.is_terminator() {
                quote! { #op }
            } else if outputs.is_empty() {
                quote! { #op; }
            } else if outputs.len() == 1 {
                quote! { let #(#outputs),* = #op; }
            } else {
                quote! { let (#(#outputs),*) = #op; }
            }
        });
        match with_braces {
            true => quote! { { #(#ops)* } },
            false => quote! { #(#ops)* },
        }
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let arguments = self.arguments.iter().map(|arg| format!("{arg}")).collect::<Vec<_>>().join(", ");
        let prefix = if f.alternate() { "    " } else { "" };
        let operations = textwrap::indent(
            &self
                .operations
                .iter()
                .map(|operation| operation.to_string(f.alternate()))
                .collect::<Vec<_>>()
                .join("\n"),
            prefix,
        );
        if !arguments.is_empty() {
            write!(f, "|{arguments}| ")?;
        };
        write!(f, "{{\n{operations}\n}}")
    }
}

impl ToTokens for Region {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_token_stream_formatted(true));
    }
}
