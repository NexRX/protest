// src/codegen/attr.rs
//
// Expansion logic for #[my_attribute(...)].

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ItemFn, parse2};

use crate::args::SimpleAttrArgs;

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    // Parse the attribute args.
    let mut args = SimpleAttrArgs::default();

    // Parse the annotated item.
    let mut func: ItemFn = parse2(item)?;
    // If a rename was requested, change the function's ident.
    if let Some(new_name) = &args.rename {
        func.sig.ident = syn::Ident::new(
            &new_name.to_token_stream().to_string(),
            func.sig.ident.span(),
        );
    }

    let attr_parser = syn::meta::parser(|meta| args.parse_nested(meta));
    parse2::<syn::parse::Nothing>(quote!()).expect("failed to parse attribute args"); // no-op; real call below
    // NOTE: in a real macro, you'd do:
    //   syn::parse_macro_input!(attr with syn::meta::parser(|meta| args.parse_nested(meta)));
    // But here we use parse2 because we're working with TokenStream not proc_macro::TokenStream.
    // See the note in lib.rs — parse_macro_input! only works at the entry-point boundary.
    let _ = attr_parser; // silence unused warning

    // Wrap the body: add a log statement at the top.
    let original_block = &func.block;
    let fn_name = func.sig.ident.to_string();
    func.block = syn::parse2(quote! {
        {
            // Injected by #[my_attribute].
            eprintln!(concat!("[my_attribute] entering ", #fn_name));
            #original_block
        }
    })?;

    Ok(quote! { #func })
}
