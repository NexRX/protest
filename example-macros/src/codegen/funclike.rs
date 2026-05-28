// src/codegen/funclike.rs
//
// Expansion logic for my_macro!(...).

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse2;

use crate::args::MappingList;

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let list: MappingList = parse2(input)?;

    // Generate a match arm for every mapping.
    let arms = list.mappings.iter().map(|m| {
        let from = &m.from;
        let to = &m.to;
        quote! { stringify!(#from) => stringify!(#to), }
    });

    Ok(quote! {
        {
            // A trivial generated expression — replace with your actual logic.
            fn __resolve(key: &str) -> &'static str {
                match key {
                    #( #arms )*
                    _ => panic!("unknown key: {}", key),
                }
            }
            __resolve
        }
    })
}
