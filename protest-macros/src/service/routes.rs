use crate::{args::RouteArgs, service::path::PathParam};
use darling::FromMeta;
use proc_macro_error2::abort;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Attribute, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Pat, PatType, Receiver};

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
            fn can_handle_request(&self, request_: &protest::RequestStream) -> bool {
                match (request_.method, request_.path_str()) {
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
                request_: protest::RequestStream,
                send: &'a mut tokio_quiche::http3::driver::OutboundFrameSender,
            ) -> protest::FutureResult<'a, (), protest::ProtestError> {
                Box::pin(async move {
                    match (request_.method, request_.path_str()) {
                        #(#route_tuples)*
                        _ => Err(protest::ProtestError::NoRoute),
                    }
                })
            }
        }
    }
}

pub(super) struct Route {
    pub args: RouteArgs,
    pub method: ImplItemFn,
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
            (protest::Method::#method_ident, _) if protest::PathMatcher::from(#path).matches(&request_.path)
        }
    }

    /// Generates a match arm that dispatches to the handler for this route.
    pub fn gen_match_handler(&self) -> TokenStream {
        let match_tuple = self.gen_match_tuple();
        let fn_ident = &self.method.sig.ident;
        let reponse_turbo_fish = match &self.method.sig.output {
            syn::ReturnType::Default => quote! {::},
            syn::ReturnType::Type(_, ty) => quote! {::<#ty>::},
        };

        let fn_call = self.gen_fn_call(fn_ident);

        // Generate a trait bound assertion that points the error at the return type span
        let response_body_assert = match &self.method.sig.output {
            syn::ReturnType::Type(_, ty) => {
                let assert_fn_ident = quote::format_ident!("_assert_response_body_{}", fn_ident);
                quote_spanned! {ty.span()=>
                    #[allow(unused)]
                    fn #assert_fn_ident() where #ty: protest::ResponseBody {}
                }
            }
            syn::ReturnType::Default => quote! {},
        };

        quote! {
            #match_tuple => {
                #response_body_assert
                let response_body = #fn_call;
                protest::Response #reponse_turbo_fish new(protest::Status::OK, response_body).send(send).await?;
                Ok(())
            }
        }
    }

    fn gen_fn_call(&self, fn_ident: &Ident) -> TokenStream {
        let conditional_await = self.method.sig.asyncness.is_some().then(|| quote! {.await});
        let inputs = RouteHandlerInputs::from_route(self);
        let pre_call = RouteHandlerInputs::gen_pre_fn_call(&inputs, &self.args);
        let fn_args = RouteHandlerInputs::gen_fn_call_inputs(&inputs);
        let receiver = RouteHandlerInputs::gen_fn_receiver_tokens(&inputs);

        quote! {
            {
                #pre_call
                #receiver #fn_ident(#fn_args)#conditional_await
            }
        }
    }

    // -------- Utilities ---------
    fn attr_is_service(attr: &Attribute) -> bool {
        attr.path().is_ident("service")
    }

    fn named_path_params(&self) -> Vec<(usize, String)> {
        self.args
            .path
            .split('/')
            .enumerate()
            .filter_map(|(path_position, segment)| {
                segment
                    .strip_prefix(':')
                    .map(|name| (path_position, name.to_string()))
            })
            .collect::<Vec<_>>()
    }
}

pub enum RouteHandlerInputs {
    /// i.e. `&self`
    Receiver(Receiver),
    Body {
        #[allow(unused)]
        fn_position: usize,
        pat_type: PatType,
    },
    PathParam(PathParam),
}

impl RouteHandlerInputs {
    pub fn from_route(route: &Route) -> Vec<Self> {
        let path_params = route.named_path_params();

        route
            .method
            .sig
            .inputs
            .iter()
            .enumerate()
            .map(|(fn_position, input)| match input {
                // match self
                FnArg::Receiver(receiver) => Self::Receiver(receiver.to_owned()),
                // match body
                FnArg::Typed(pat_type) if Self::is_body(pat_type) => Self::Body {
                    fn_position,
                    pat_type: pat_type.clone(),
                },
                // match path
                FnArg::Typed(pat_type) if Self::is_path_param(pat_type, &path_params) => Self::PathParam(PathParam::from_pat_type(fn_position, pat_type, &path_params)),
                unsupported => abort!(unsupported, "Unsupported arguement because it couldn't be identified as one of the follow args: `self`, `body: T` / `#[body] arg: T`, `path_name: T` / `#[path(\"name\")] arg: T`"),
            })
            .collect()
    }

    pub fn gen_pre_fn_call(inputs: &[Self], args: &RouteArgs) -> TokenStream {
        inputs
            .iter()
            .map(|input| match input {
                RouteHandlerInputs::Receiver(_) => quote! {},
                RouteHandlerInputs::Body {
                    pat_type,
                    ..
                } => {
                    //
                    let name =  if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        &pat_ident.ident
                    } else {
                        abort!(pat_type, "Expect body fn argument to be known by an Ident i.e. `body: T` where `body` is the ident");
                    };
                    let capacitity = match args.alloc_body {
                        true => quote! {(&request_.headers.content_length).as_ref().map(|v| *v).unwrap_or_default()},
                        false => quote! {0},
                    };

                    let body_ty = &pat_type.ty;
                    quote! {
                        let capacitity_ = #capacitity;
                        let request_ = request_.into_buffered_typed::<#body_ty>(capacitity_).await?;
                        let #name = request_.body;
                    }
                }
                RouteHandlerInputs::PathParam(path_param) => {
                    let name = &path_param.name;
                    let name_str = path_param.name.to_string();
                    let param_ty = &path_param.fn_type.ty;
                    let path_param_position = path_param.path_position;

                    let raw_name = format_ident!("__protest_raw_{}", name);
                    let conversion = match &**param_ty {
                        syn::Type::Reference(type_ref) if matches!(&*type_ref.elem, syn::Type::Path(p) if p.path.is_ident("str")) => quote! { let #name: #param_ty = &*#raw_name; },
                        param_ty => {
                            let owned_ty = match param_ty {
                                syn::Type::Reference(v) => &*v.elem,
                                ty => ty,
                            };
                            let conditional_borrow = match param_ty {
                                syn::Type::Reference(_) => quote! { let #name = &#name; },
                                _ => quote!(),
                            };
                            quote! {
                                let __cloned_value = #raw_name.clone();
                                let #name = <String as TryInto<#owned_ty>>::try_into(#raw_name).map_err(|err| protest::RequestError::Invalid {
                                    name: #name_str.into(),
                                    kind: protest::RequestParamKind::Path,
                                    raw_value: Some(__cloned_value),
                                    conversion_type: Some(stringify!(#param_ty).into()),
                                    message: err.to_string(),
                                })?;
                                #conditional_borrow
                            }
                        }
                    };

                    quote! {
                        let #raw_name: String = request_
                            .path
                            .split('/')
                            .nth(#path_param_position)
                            .ok_or(protest::RequestError::Invalid {
                                name: #name_str.into(),
                                kind: protest::RequestParamKind::Path,
                                raw_value: None,
                                conversion_type: None,
                                message: format!("No path parameter found at position {}", #path_param_position),
                            })?
                            .into();
                        #conversion
                    }
                },
            })
            .collect()
    }

    pub fn gen_fn_call_inputs(inputs: &[Self]) -> TokenStream {
        inputs
            .iter()
            .filter(|v| !matches!(v, Self::Receiver(_)))
            .map(|input| {
                let name = input.name();
                quote! {#name}
            })
            .reduce(|a, b| quote! {#a, #b})
            .unwrap_or_default()
    }

    pub fn gen_fn_receiver_tokens(inputs: &[Self]) -> TokenStream {
        inputs
            .iter()
            .find(|v| matches!(v, Self::Receiver(_)))
            .map_or(quote! { Self::}, |v| match v {
                Self::Receiver(f) => {
                    let self_token = &f.self_token;
                    quote! {#self_token.}
                }
                _ => unreachable!(),
            })
    }

    pub fn name(&self) -> Ident {
        match self {
            RouteHandlerInputs::Receiver(r) => Ident::new("self", r.self_token.span),
            RouteHandlerInputs::Body { pat_type, .. } => {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    pat_ident.ident.clone()
                } else {
                    abort!(
                        pat_type,
                        "Expect body fn argument to be known by an Ident i.e. `body: T` where `body` is the ident"
                    );
                }
            }
            RouteHandlerInputs::PathParam(path_param) => path_param.name.clone(),
        }
    }

    pub fn is_body(pat_type: &PatType) -> bool {
        if let Pat::Ident(ident) = &*pat_type.pat {
            let is_attributed_body = ident.attrs.iter().any(|attr| attr.path().is_ident("body"));
            let is_ident_body = ident.ident == "body";
            is_attributed_body || is_ident_body
        } else {
            false
        }
    }

    pub fn is_path_param(pat_type: &PatType, path_params: &[(usize, String)]) -> bool {
        if let Pat::Ident(ident) = &*pat_type.pat {
            let is_attributed_body = ident.attrs.iter().any(|attr| attr.path().is_ident("path"));
            let is_ident_path_param = path_params.iter().any(|(_, name)| ident.ident == name);
            is_attributed_body || is_ident_path_param
        } else {
            false
        }
    }
}
