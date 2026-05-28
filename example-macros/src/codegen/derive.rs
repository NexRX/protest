// src/codegen/derive.rs
//
// Expansion logic for #[derive(MyTrait)].
// Separated from lib.rs so it can be tested without going through proc_macro.

use darling::FromDeriveInput;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::args::{ContainerArgs, FieldArgs, named_fields};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    // Parse the container-level attributes using darling.
    let args = ContainerArgs::from_derive_input(&input)
        .map_err(|e| syn::Error::new_spanned(&input.ident, e.to_string()))?;

    let ident = &args.ident;
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

    // Collect field info.
    let fields: Vec<&FieldArgs> = named_fields(&args.data).filter(|f| !f.skip).collect();

    // Generate one method arm per field.
    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("named field"))
        .collect();

    let field_renames: Vec<_> = fields
        .iter()
        .map(|f| {
            f.rename
                .clone()
                .unwrap_or_else(|| f.ident.as_ref().unwrap().to_string().to_snake_case())
        })
        .collect();

    // Optionally honour a top-level rename for the struct.
    let struct_name = args
        .rename
        .clone()
        .unwrap_or_else(|| ident.to_string());

    // Build the impl block.
    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics MyTrait for #ident #ty_generics #where_clause {
            fn type_name() -> &'static str {
                #struct_name
            }

            fn field_names() -> &'static [&'static str] {
                &[ #( #field_renames ),* ]
            }

            fn describe(&self) -> String {
                let mut parts = Vec::new();
                #(
                    parts.push(format!(
                        "{} = {:?}",
                        #field_renames,
                        &self.#field_names
                    ));
                )*
                format!("{} {{ {} }}", #struct_name, parts.join(", "))
            }
        }
    };

    Ok(expanded)
}

// ---------------------------------------------------------------------------
// Helper — split generics and add a bound to every type parameter
// ---------------------------------------------------------------------------

/// Adds `bound` to every type parameter in `generics`.
///
/// Example: add_trait_bounds(generics, parse_quote!(MyTrait))
pub fn add_trait_bounds(mut generics: syn::Generics, bound: syn::TypeParamBound) -> syn::Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut ty) = *param {
            ty.bounds.push(bound.clone());
        }
    }
    generics
}

// ---------------------------------------------------------------------------
// Helper — generate a unique private identifier
// ---------------------------------------------------------------------------

pub fn private_ident(base: &syn::Ident) -> syn::Ident {
    format_ident!("__my_macro_{}", base)
}

// ---------------------------------------------------------------------------
// Helper — pretty-print a TokenStream via prettyplease
// ---------------------------------------------------------------------------

pub fn pretty_print(ts: TokenStream) -> String {
    let file: syn::File = syn::parse2(ts).expect("generated code was not valid Rust");
    prettyplease::unparse(&file)
}
