mod routes;
mod path;

use crate::service::routes::Routes;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl, parse2};

pub fn expand(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let impl_block = parse2::<ItemImpl>(item)?;

    let service_ty = &impl_block.self_ty;

    let attrs = &impl_block.attrs;
    let generics = &impl_block.generics;
    let trait_tokens = trait_tokens(&impl_block);
    let cleaned_items = gen_cleaned_items(&impl_block);

    let routes = Routes::from_impl(&impl_block);
    let fn_can_handle_request = routes.gen_can_handle_request();
    let fn_len = routes.gen_len();
    let fn_handle_request = routes.gen_handle_request();

    Ok(quote! {
        use protest::ResponseSender as _;

        // Original impl block, `#[service]` attrs removed from methods.
        #(#attrs)*
        impl #generics #trait_tokens #service_ty {
            #(#cleaned_items)*
        }

        impl protest::TRouter for #service_ty {
            #fn_can_handle_request
            #fn_len
            #fn_handle_request
        }
    })
}

fn gen_cleaned_items(impl_block: &ItemImpl) -> Vec<ImplItem> {
    impl_block
        .items
        .iter()
        .map(|item| match item {
            ImplItem::Fn(f) => {
                let mut f = f.clone();
                f.attrs.retain(|a| !a.path().is_ident("service"));
                ImplItem::Fn(f)
            }
            other => other.clone(),
        })
        .collect()
}

fn trait_tokens(impl_block: &ItemImpl) -> Option<TokenStream> {
    impl_block
        .trait_
        .as_ref()
        .map(|(bang, path, for_)| quote! { #bang #path #for_ })
}
