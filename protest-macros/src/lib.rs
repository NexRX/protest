pub(crate) mod args;
mod service;

use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr2 = proc_macro2::TokenStream::from(attr);
    let item2 = proc_macro2::TokenStream::from(item);

    match crate::service::expand(attr2, item2) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
