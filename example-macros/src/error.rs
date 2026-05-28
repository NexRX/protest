// src/error.rs
//
// Error handling in proc macros is tricky because you want to:
//   a) emit errors with correct source spans so the user sees the right line.
//   b) keep going after the first error to surface as many problems as possible.
//   c) not panic (panics produce ugly "proc macro panicked" messages).
//
// This module shows patterns for all three goals.

// ---------------------------------------------------------------------------
// Pattern 1 — syn::Error  (always available, no extra deps)
// ---------------------------------------------------------------------------
//
// `syn::Error` carries a Span, can be combined with `combine`, and converts
// to a TokenStream via `.into_compile_error()`.
//
// Best for: errors discovered during parsing.

pub fn check_ident_is_pascal(ident: &syn::Ident) -> syn::Result<()> {
    let name = ident.to_string();
    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            ident,
            format!("expected PascalCase identifier, got `{name}`"),
        ))
    }
}

// Combining multiple errors so all are shown at once:
pub fn check_all_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
) -> syn::Result<()> {
    let mut errors: Option<syn::Error> = None;

    for field in fields {
        if let Some(ident) = &field.ident {
            if let Err(e) = check_ident_is_pascal(ident) {
                match &mut errors {
                    None => errors = Some(e),
                    Some(prev) => prev.combine(e),
                }
            }
        }
    }

    errors.map_or(Ok(()), Err)
}

// ---------------------------------------------------------------------------
// Pattern 2 — proc-macro-error2
// ---------------------------------------------------------------------------
//
// Gives you abort!(), emit_error!(), emit_warning!() that work like panics
// but produce proper compiler diagnostics.
//
// Usage: annotate your entry point with #[proc_macro_error] and then use
// abort!/emit_error! anywhere in the call stack.
//
// In lib.rs:
//   #[proc_macro_error::proc_macro_error]
//   #[proc_macro_derive(MyTrait)]
//   pub fn derive_my_trait(input: TokenStream) -> TokenStream { ... }

use proc_macro2::Span;

pub fn example_abort(span: Span) {
    // This exits immediately with a compiler error at `span`.
    proc_macro_error2::abort!(span, "something went wrong"; help = "try doing X instead");
}

pub fn example_emit(span: Span) {
    // This records an error but continues — use at the end of iteration.
    proc_macro_error2::emit_error!(span, "field is invalid");
    // After all emits, proc_macro_error2 will emit all of them at once.
}

// ---------------------------------------------------------------------------
// Pattern 3 — manyhow  (combines syn::Error + anyhow ergonomics)
// ---------------------------------------------------------------------------
//
// manyhow lets you write `fn expand(...) -> manyhow::Result<TokenStream>`
// and use `?` on both syn::Error and any std::error::Error.
// It also integrates with darling and emits all errors collected so far.
//
// In lib.rs entry point:
//   #[manyhow(proc_macro_derive(MyTrait))]
//   pub fn derive_my_trait(input: syn::DeriveInput) -> manyhow::Result<TokenStream2> {
//       // syn::parse is automatic, return type is clean
//   }
//
// See codegen/ modules for actual usage.

// ---------------------------------------------------------------------------
// Pattern 4 — Accumulating errors with a collector
// ---------------------------------------------------------------------------
//
// When you want to keep going after the first error (e.g. validating every
// field) without proc-macro-error2's global state:

pub struct ErrorAccumulator(Vec<syn::Error>);

impl ErrorAccumulator {
    pub fn new() -> Self { Self(Vec::new()) }

    pub fn push(&mut self, e: syn::Error) {
        self.0.push(e);
    }

    pub fn push_spanned<T: quote::ToTokens>(&mut self, tokens: T, msg: impl std::fmt::Display) {
        self.0.push(syn::Error::new_spanned(tokens, msg));
    }

    /// Returns Ok(()) if no errors were accumulated, otherwise combines them
    /// all into a single syn::Error.
    pub fn finish(self) -> syn::Result<()> {
        let mut iter = self.0.into_iter();
        let Some(mut combined) = iter.next() else { return Ok(()) };
        for e in iter { combined.combine(e); }
        Err(combined)
    }
}
