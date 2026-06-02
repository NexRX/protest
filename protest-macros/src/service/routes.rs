use crate::args::RouteArgs;
use darling::FromMeta;
use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, FnArg, ImplItem, ImplItemFn, ItemImpl, PatType};

pub struct Routes(Vec<Route>);

impl Routes {
    // -------- Constructors ---------

    pub fn from_impl(impl_block: &ItemImpl) -> Self {
        Self(
            impl_block
                .items
                .iter()
                .filter(|v| match v {
                    ImplItem::Const(impl_item_const) => {
                        impl_item_const.attrs.iter().any(Route::attr_is_service)
                    }
                    ImplItem::Fn(impl_item_fn) => {
                        impl_item_fn.attrs.iter().any(Route::attr_is_service)
                    }
                    ImplItem::Type(impl_item_type) => {
                        impl_item_type.attrs.iter().any(Route::attr_is_service)
                    }
                    ImplItem::Macro(impl_item_macro) => {
                        impl_item_macro.attrs.iter().any(Route::attr_is_service)
                    }
                    ImplItem::Verbatim(_token_stream) => false,
                    _ => false,
                })
                .map(|v| match v {
                    ImplItem::Fn(impl_item_fn) => Route::from_impl_fn(impl_item_fn),
                    span => abort!(span, "unsupported item in impl: {:?}", span),
                })
                .collect(),
        )
    }

    // -------- Codegen ---------

    pub fn gen_can_handle_request(&self) -> TokenStream {
        let route_tuples: Vec<TokenStream> = self.0.iter().map(Route::gen_match_tuple).collect();

        quote! {
            fn can_handle_request(&self, request: &protest::RequestStream) -> bool {
                match (request.method, request.path_str()) {
                    #(#route_tuples => true,)*
                    _ => false,
                }
            }
        }
    }

    pub fn gen_len(&self) -> TokenStream {
        let len = self.0.len();
        quote! {
            fn len(&self) -> usize {
                #len
            }
        }
    }

    pub fn gen_handle_request(&self) -> TokenStream {
        let route_tuples: Vec<TokenStream> = self.0.iter().map(Route::gen_match_handler).collect();

        quote! {
            fn handle_request<'a>(
                &'a self,
                request: protest::RequestStream,
                send: &'a mut tokio_quiche::http3::driver::OutboundFrameSender,
            ) -> protest::FutureResult<'a, (), protest::ProtestError> {
                Box::pin(async move {
                    match (request.method, request.path_str()) {
                        #(#route_tuples)*
                        _ => Err(protest::ProtestError::NoRoute),
                    }
                })
            }
        }
    }
}

pub(super) struct Route {
    args: RouteArgs,
    method: ImplItemFn,
}

impl Route {
    // -------- Constructors ---------
    fn from_impl_fn(method: &ImplItemFn) -> Self {
        for attr in &method.attrs {
            if attr.path().is_ident("service") {
                let meta = attr.meta.clone();
                let args = RouteArgs::from_meta(&meta)
                    .map_err(|e| abort!(method, "invalid service attribute: {:?}", e))
                    .unwrap();

                return Self {
                    args,
                    method: method.clone(),
                };
            }
        }
        abort!(method, "no service attribute found")
    }

    // -------- Codegen ---------

    /// Generates a tokenstream that expands to a tuple of (Method, path).
    pub fn gen_match_tuple(&self) -> TokenStream {
        let path = &self.args.path;
        let method_ident = &self.args.method;
        quote! {
            (protest::Method::#method_ident, #path)
        }
    }

    /// Generates a match arm that dispatches to the handler for this route.
    pub fn gen_match_handler(&self) -> TokenStream {
        let match_tuple = self.gen_match_tuple();
        let fn_ident = &self.method.sig.ident;
        let conditional_await = self.method.sig.asyncness.is_some().then(|| quote! {.await});
        let reponse_turbo_fish = match &self.method.sig.output {
            syn::ReturnType::Default => quote! {::},
            syn::ReturnType::Type(_, ty) => quote! {::<#ty>::},
        };

        match self.body_ty() {
            Some(body_ty) => {
                let fn_call = if self.has_receiver() {
                    quote! { self.#fn_ident(request.body)#conditional_await }
                } else {
                    quote! { Self::#fn_ident(request.body)#conditional_await }
                };
                let body_ty = &body_ty.ty;
                quote! {
                    #match_tuple => {
                        let request = request.into_buffered_typed::<#body_ty>(0).await?;
                        let response_body = #fn_call;
                        protest::Response #reponse_turbo_fish new(protest::Status::OK, response_body).send(send).await?;
                        Ok(())
                    }
                }
            }
            None => {
                let fn_call = if self.has_receiver() {
                    quote! { self.#fn_ident()#conditional_await }
                } else {
                    quote! { Self::#fn_ident()#conditional_await }
                };
                quote! {
                    #match_tuple => {
                        let response_body = #fn_call;
                        protest::Response::new(protest::Status::OK, response_body).send(send).await?;
                        Ok(())
                    }
                }
            }
        }
    }

    // -------- Utilities ---------
    fn attr_is_service(attr: &Attribute) -> bool {
        attr.path().is_ident("service")
    }

    fn body_ty(&self) -> Option<&PatType> {
        let args = self.named_args();
        if args.len() > 1 {
            let first_extra = args.iter().skip(1).next().unwrap();
            abort!(
                first_extra.ty,
                "Only none or one arg is supported currently and it must be the request body"
            );
        }
        args.first().map(|v| &**v)
    }

    fn has_receiver(&self) -> bool {
        self.method
            .sig
            .inputs
            .iter()
            .any(|arg| matches!(arg, FnArg::Receiver(_)))
    }

    fn named_args(&self) -> Vec<&PatType> {
        self.method
            .sig
            .inputs
            .iter()
            .filter_map(|v| match v {
                FnArg::Receiver(_) => None,
                FnArg::Typed(pat_type) => Some(pat_type),
            })
            .collect::<Vec<_>>()
    }
}
