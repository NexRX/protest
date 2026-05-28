// src/lib.rs
//
// Proc macro starter library.
//
// A proc macro crate's lib.rs ONLY exposes the public macro entry points.
// All the real logic lives in submodules so it can be unit-tested without
// going through the full compiler harness.

use proc_macro::TokenStream;

mod args;        // Attribute / argument parsing (darling + manual syn::parse)
mod codegen;     // Code generation helpers (quote + prettyplease)
mod error;       // Error handling helpers (proc-macro-error2 + manyhow)

// ---------------------------------------------------------------------------
// Example 1 — Derive macro
// ---------------------------------------------------------------------------
//
// Usage:
//   #[derive(MyTrait)]
//   struct Foo { bar: String, baz: u32 }
//
#[proc_macro_derive(MyTrait, attributes(my_trait))]
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    // Parse the input as a syn::DeriveInput (struct / enum / union).
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Delegate to the implementation so we can unit-test it.
    match crate::codegen::derive::expand(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

// ---------------------------------------------------------------------------
// Example 2 — Attribute macro
// ---------------------------------------------------------------------------
//
// Usage:
//   #[my_attribute(rename = "hello", version = 2)]
//   fn do_something() { ... }
//
#[proc_macro_attribute]
pub fn my_attribute(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr2 = proc_macro2::TokenStream::from(attr);
    let item2 = proc_macro2::TokenStream::from(item);

    match crate::codegen::attr::expand(attr2, item2) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

// ---------------------------------------------------------------------------
// Example 3 — Function-like macro
// ---------------------------------------------------------------------------
//
// Usage:
//   let x = my_macro!(some, custom, syntax 42);
//
#[proc_macro]
pub fn my_macro(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);

    match crate::codegen::funclike::expand(input2) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
