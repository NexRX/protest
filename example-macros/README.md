# proc-macro-starter
 
A batteries-included template for writing Rust procedural macros. Clone it,
rename the crate, and start hacking. Everything below is working, tested code —
not placeholder comments.
 
---
 
## Quick start
 
```sh
# Clone / copy the template
cp -r proc-macro-starter my-awesome-macro
cd my-awesome-macro
 
# Vendor dependencies for offline use (e.g. on a plane ✈️)
cargo vendor
# Cargo prints a snippet to add to .cargo/config.toml — paste it in.
# Then every subsequent build works without internet access.
 
# Build
cargo build
 
# Test
cargo test
```
 
---
 
## Dependency guide
 
### Core trio
 
| Crate | Why |
|---|---|
| `proc-macro2` | A `TokenStream` that works in both proc-macro and regular contexts, so your expansion logic can be unit-tested. |
| `syn` | Parses a `TokenStream` into a typed Rust AST. You will use this for almost everything. |
| `quote` | Turns Rust expressions / variables back into token streams via the `quote! { }` macro. |
 
#### `syn` features to know
 
```toml
syn = { version = "2", features = ["full", "extra-traits", "visit", "visit-mut", "fold"] }
```
 
| Feature | What it unlocks |
|---|---|
| `full` | Parsing for all Rust syntax — expressions, statements, items. Without it you only get types and paths. |
| `extra-traits` | `Debug` / `PartialEq` / `Hash` on every AST node. **Turn this on while developing** — `dbg!()` on a `syn::Expr` is invaluable. |
| `visit` | The `Visit` trait: a read-only walk of the AST. Implement it to collect information. |
| `visit-mut` | The `VisitMut` trait: a mutable walk. Use it to transform the AST in-place. |
| `fold` | The `Fold` trait: consume an AST node and return a transformed one. Similar to `VisitMut` but ownership-based. |
 
---
 
### Attribute / argument parsing
 
#### Darling — derive-based attribute parsing
 
Darling lets you describe your macro's attribute API as a Rust struct and
derives the parsing for you.
 
```rust
use darling::FromDeriveInput;
 
#[derive(FromDeriveInput)]
#[darling(attributes(my_trait), supports(struct_named))]
pub struct MyArgs {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
 
    #[darling(default)]
    pub rename: Option<String>,
 
    #[darling(default)]
    pub skip_debug: bool,
}
 
// In your derive entry point:
let args = MyArgs::from_derive_input(&input)?;
println!("{}", args.rename.unwrap_or_default());
```
 
Per-field attributes use `FromField`:
 
```rust
#[derive(darling::FromField)]
#[darling(attributes(my_trait))]
pub struct FieldArgs {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
 
    #[darling(default)]
    pub skip: bool,
}
```
 
Darling emits nice compiler errors for unknown keys. The `suggestions` feature
even suggests corrections for typos.
 
#### Deluxe — alternative derive-based parsing
 
Deluxe has a slightly different API that some find easier for complex schemas.
 
```rust
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(my_trait))]
struct DeluxeArgs {
    #[deluxe(default)]
    rename: Option<String>,
 
    #[deluxe(default = true)]
    generate_display: bool,
}
 
// Usage — mutates the attribute list, extracting matched attrs:
let args: DeluxeArgs = deluxe::extract_attributes(&mut derive_input)?;
```
 
#### Manual `syn::Parse` — for custom syntax
 
When your macro takes non-key-value syntax (e.g. `my_macro!(Foo => bar, Baz => qux)`):
 
```rust
use syn::{parse::{Parse, ParseStream}, punctuated::Punctuated, Token};
 
struct Mapping {
    from: syn::Ident,
    _arrow: Token![=>],
    to: syn::Ident,
}
 
impl Parse for Mapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self { from: input.parse()?, _arrow: input.parse()?, to: input.parse()? })
    }
}
 
struct MappingList {
    mappings: Punctuated<Mapping, Token![,]>,
}
 
impl Parse for MappingList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self { mappings: Punctuated::parse_terminated(input)? })
    }
}
```
 
#### `syn::meta::parser` — lightweight key-value parsing
 
For `#[proc_macro_attribute]` macros where you control the attr `TokenStream`
directly, without pulling in darling:
 
```rust
#[derive(Default)]
struct AttrArgs { rename: Option<syn::LitStr>, skip: bool }
 
impl AttrArgs {
    fn parse_nested(&mut self, meta: syn::meta::ParseNestedMeta) -> syn::Result<()> {
        if meta.path.is_ident("rename") {
            self.rename = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("skip") {
            self.skip = true;
        } else {
            return Err(meta.error("unknown key"));
        }
        Ok(())
    }
}
 
// In the macro entry point:
let mut args = AttrArgs::default();
parse_macro_input!(attr with syn::meta::parser(|meta| args.parse_nested(meta)));
```
 
---
 
### Code generation
 
#### `quote!` basics
 
```rust
use quote::quote;
use proc_macro2::TokenStream;
 
fn generate(name: &syn::Ident, fields: &[syn::Ident]) -> TokenStream {
    quote! {
        impl MyTrait for #name {
            fn fields() -> &'static [&'static str] {
                &[ #( stringify!(#fields) ),* ]
            }
        }
    }
}
```
 
Key `quote!` interpolation syntax:
 
| Syntax | Meaning |
|---|---|
| `#ident` | Interpolate a single value that implements `ToTokens`. |
| `#( #items ),*` | Repeat `#items` separated by commas. |
| `#( #items );*` | Repeat separated by semicolons. |
| `#( let #names = #vals; )*` | Repeat multiple variables together (zipped). |
 
#### `format_ident!` — dynamic identifiers
 
```rust
use quote::format_ident;
let getter = format_ident!("get_{}", field_name); // → get_foo
let private = format_ident!("__impl_{}", struct_name); // → __impl_Foo
```
 
#### `heck` — case conversion
 
```rust
use heck::{ToSnakeCase, ToPascalCase, ToShoutySnakeCase};
"MyStruct".to_snake_case()        // → "my_struct"
"my_field".to_pascal_case()       // → "MyField"
"my_const".to_shouty_snake_case() // → "MY_CONST"
```
 
#### `prettyplease` — format generated code
 
Useful for snapshot tests and debug output:
 
```rust
let ts: proc_macro2::TokenStream = quote! { fn foo() { let x = 1 + 2; } };
let file: syn::File = syn::parse2(ts).unwrap();
let pretty = prettyplease::unparse(&file);
println!("{pretty}");
// fn foo() {
//     let x = 1 + 2;
// }
```
 
---
 
### Error handling
 
#### Option 1 — `syn::Error` (always available)
 
```rust
return Err(syn::Error::new_spanned(&field.ident, "this field is invalid"));
```
 
Combine multiple errors so the user sees all of them at once:
 
```rust
let mut errors: Option<syn::Error> = None;
for field in &fields {
    if let Err(e) = validate(field) {
        match &mut errors { None => errors = Some(e), Some(p) => p.combine(e) }
    }
}
if let Some(e) = errors { return Err(e); }
```
 
Convert to tokens at the entry point:
 
```rust
match expand(input) {
    Ok(ts) => ts.into(),
    Err(e) => e.into_compile_error().into(),
}
```
 
#### Option 2 — `proc-macro-error2`
 
Annotate your entry point and use `abort!` anywhere in the call stack:
 
```rust
#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(MyTrait)]
pub fn derive(input: TokenStream) -> TokenStream { ... }
 
// Deep in the call stack:
proc_macro_error2::abort!(span, "bad value"; help = "expected one of: foo, bar");
proc_macro_error2::emit_warning!(span, "this is deprecated");
```
 
#### Option 3 — `manyhow`
 
Cleans up the entry point: no `match`, no `.into()`, full `?` support:
 
```rust
#[manyhow::manyhow(proc_macro_derive(MyTrait, attributes(my_trait)))]
fn derive(input: syn::DeriveInput) -> manyhow::Result<TokenStream2> {
    let args = MyArgs::from_derive_input(&input)?; // darling error → ?
    // ...
    Ok(quote! { ... })
}
```
 
---
 
### Modelling macro arguments — the mental model
 
Think of macro arguments in three layers:
 
```
1. TokenStream (raw)
      ↓  parse with syn / darling / deluxe
2. Argument struct (typed, validated)
      ↓  pass to codegen functions
3. quote! { } (output TokenStream)
```
 
**Design your argument struct first.** Before writing any `impl Parse`, write
the Rust struct that represents what you _want_ the user to express:
 
```rust
// What the user writes:
//   #[my_trait(rename = "Foo", version = 2)]
//   struct Bar { #[my_trait(skip)] x: u32 }
 
struct ContainerArgs {
    rename: Option<String>,
    version: u8,          // default = 1
}
struct FieldArgs {
    skip: bool,
}
```
 
Then choose a parser (darling / deluxe / manual) based on whether the user
syntax is key-value pairs or something more exotic.
 
**Rule of thumb:**
- Key-value attribute args on items → darling or deluxe.
- Custom DSL in function-like macros → manual `impl Parse`.
- Simple true/false flags or one rename → `syn::meta::parser` (no extra dep).
---
 
### Testing
 
#### `trybuild` — compile-error tests
 
Create files in `tests/ui/`:
 
```
tests/
  ui/
    pass/
      basic_struct.rs       ← must compile
    fail/
      missing_field.rs      ← must fail with this stderr:
      missing_field.stderr  ← exact expected error text
```
 
```rust
// tests/ui_tests.rs
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
```
 
Run with `cargo test`. On the first run against new fail-cases, set
`TRYBUILD=overwrite` to generate the `.stderr` files automatically.
 
#### `macrotest` — expansion snapshot tests
 
```rust
// tests/expand_tests.rs
#[test]
fn expand() {
    macrotest::expand("tests/expand/*.rs");
}
```
 
Create `tests/expand/basic.rs` with an invocation of your macro, then run
`cargo test` with `MACROTEST=overwrite` to generate `basic.expanded.rs`.
Commit both files — CI catches unintended expansion changes.
 
#### `insta` — snapshot testing with pretty output
 
```rust
#[test]
fn snapshot_derive() {
    let input: syn::DeriveInput = syn::parse_quote! {
        struct Foo { bar: String }
    };
    let ts = crate::codegen::derive::expand(input).unwrap();
    let pretty = crate::codegen::derive::pretty_print(ts);
    insta::assert_snapshot!(pretty);
}
```
 
Run `cargo insta review` to review and accept new snapshots.
 
#### `rstest` — parametric tests
 
```rust
use rstest::rstest;
 
#[rstest]
#[case("snake_case", "SnakeCase")]
#[case("my_struct", "MyStruct")]
fn test_case_conversion(#[case] input: &str, #[case] expected: &str) {
    use heck::ToPascalCase;
    assert_eq!(input.to_pascal_case(), expected);
}
```
 
---
 
### Workflow tips
 
#### `cargo expand`
 
Install with `cargo install cargo-expand`. Shows the full expansion of any
macro invocation — the fastest way to debug generated code:
 
```sh
cargo expand          # expand everything in lib.rs
cargo expand MyStruct # expand just the derive on MyStruct
```
 
#### `dbg!()` on AST nodes
 
With `syn = { features = ["extra-traits"] }` enabled, every node implements
`Debug`. Put `eprintln!("{:#?}", &input)` in your expansion function while
developing; remove before publishing.
 
#### Span discipline
 
Always prefer `Error::new_spanned(tokens, msg)` over `Error::new(span, msg)`.
`new_spanned` highlights the whole relevant token tree, not just one location.
 
#### The `dummy` pattern
 
When your macro fails, return some dummy output so downstream code still
compiles and the user sees _all_ errors, not just the first:
 
```rust
match expand(input) {
    Ok(ts) => ts.into(),
    Err(e) => {
        let dummy = quote! { struct __DummyForErrorRecovery; };
        let error = e.into_compile_error();
        TokenStream::from(quote! { #error #dummy })
    }
}
```
 
---
 
## Project layout
 
```
src/
  lib.rs           ← pub proc_macro entry points only
  args.rs          ← argument / attribute parsing (darling, deluxe, manual)
  error.rs         ← error accumulation helpers
  codegen/
    mod.rs
    derive.rs      ← #[derive(MyTrait)] expansion
    attr.rs        ← #[my_attribute] expansion
    funclike.rs    ← my_macro!() expansion
tests/
  ui/              ← trybuild pass / fail tests
  expand/          ← macrotest expansion snapshots
```
 
---
 
## License
 
MIT OR Apache-2.0
