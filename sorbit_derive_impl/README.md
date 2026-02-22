# Sorbit derive macro implementation

This crate contains the implementation of the derive macros used by [sorbit](https://github.com/petiaccja/sorbit), and is not meant to be used on its own.

This crate is split out from the `sorbit_derive` crate, but isn't itself a procedural macro crate. The only reasons for this split is because procedural macro crates behave rather poorly with rust-analyzer at the moment, making development cumbersome.

This crate contains:
- Parsing of derive inputs and sorbit's custom attributes
- An AST that can be parsed from `syn`'s input
- An SSA IR that is lowered from the AST
- Generation of Rust code from the SSA IR

The crate is largely undocumented, and its API might change significantly.