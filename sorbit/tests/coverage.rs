use std::ffi::OsStr;
use std::path::Path;
use std::{env, fs};

use proc_macro2::TokenStream;
use runtime_macros::emulate_derive_macro_expansion;

use sorbit_derive_impl::DeriveObject;

#[test]
fn code_coverage_runtime_macros() {
    let test_dir = env::current_dir().unwrap().join("tests").join("derive");
    let num_files = for_each_rust_src_file(&test_dir, &expand_src_file).unwrap();
    assert!(num_files != 0, "no files expanded, the test configuration is wrong");
}

fn expand_src_file(file: &Path) {
    {
        let file = fs::File::open(file).unwrap();
        emulate_derive_macro_expansion(file, &[("Serialize", expand_serialize)]).unwrap();
    }
    {
        let file = fs::File::open(file).unwrap();
        emulate_derive_macro_expansion(file, &[("Deserialize", expand_deserialize)]).unwrap();
    }
    {
        let file = fs::File::open(file).unwrap();
        emulate_derive_macro_expansion(file, &[("PackInto", expand_pack_into)]).unwrap();
    }
    {
        let file = fs::File::open(file).unwrap();
        emulate_derive_macro_expansion(file, &[("UnpackFrom", expand_unpack_from)]).unwrap();
    }
}

fn expand_serialize(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse2(input).unwrap();
    let derive_object = DeriveObject::parse(derive_input).unwrap();
    derive_object.derive_serialize()
}

fn expand_deserialize(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse2(input).unwrap();
    let derive_object = DeriveObject::parse(derive_input).unwrap();
    derive_object.derive_deserialize()
}

fn expand_pack_into(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse2(input).unwrap();
    let derive_object = DeriveObject::parse(derive_input).unwrap();
    derive_object.derive_pack_into()
}

fn expand_unpack_from(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse2(input).unwrap();
    let derive_object = DeriveObject::parse(derive_input).unwrap();
    derive_object.derive_unpack_from()
}

fn for_each_rust_src_file(dir: &Path, f: &impl Fn(&Path)) -> std::io::Result<u64> {
    let mut num_files = 0;
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            num_files += for_each_rust_src_file(&path, f)?;
        }
    } else if dir.is_file() && dir.extension() == Some(OsStr::new("rs")) {
        f(dir);
        num_files += 1;
    }
    Ok(num_files)
}
