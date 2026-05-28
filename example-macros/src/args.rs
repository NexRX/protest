// src/args.rs
//
// Everything to do with parsing macro arguments / attributes.
//
// Three approaches are shown:
//   1. Darling — derive-based, best for attribute arguments on derive macros.
//   2. Manual syn::parse_macro_input! — best when you need unusual syntax.
//   3. Deluxe — alternative derive-based approach.
//
// Pick whichever fits your macro's input shape.

// ---------------------------------------------------------------------------
// 1. Darling  — structured attribute parsing
// ---------------------------------------------------------------------------
//
// For a derive macro whose fields can be annotated like:
//
//   #[derive(MyTrait)]
//   #[my_trait(rename = "Foo", skip_debug)]
//   struct S {
//       #[my_trait(rename = "bar")]
//       field: String,
//   }

use darling::{ast, util, FromDeriveInput, FromField, FromVariant};

/// Top-level container attributes — applied to the struct/enum itself.
///
/// Darling reads `#[my_trait(...)]` and maps it to the fields below.
/// Unrecognised keys produce a compiler error automatically.
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(my_trait),   // which outer attribute to read
    supports(struct_named, enum_any), // which Rust items are supported
    forward_attrs(allow, doc, cfg),   // pass-through attrs to generated code
)]
pub struct ContainerArgs {
    /// The name the macro sees (syn's Ident for the struct/enum).
    pub ident: syn::Ident,

    /// Generics on the container — you'll need these for impl<T> blocks.
    pub generics: syn::Generics,

    /// The fields / variants, each parsed via `FieldArgs` / `VariantArgs`.
    pub data: ast::Data<VariantArgs, FieldArgs>,

    // ---- Custom options ----

    /// `#[my_trait(rename = "NewName")]` — override the generated name.
    #[darling(default)]
    pub rename: Option<String>,

    /// `#[my_trait(skip_debug)]` — boolean flag, defaults to false.
    #[darling(default)]
    pub skip_debug: bool,

    /// `#[my_trait(version = 2)]` — integer option with a default.
    #[darling(default = "default_version")]
    pub version: u8,
}

fn default_version() -> u8 { 1 }

/// Per-field attributes.
#[derive(Debug, FromField)]
#[darling(attributes(my_trait), forward_attrs(allow, doc, cfg))]
pub struct FieldArgs {
    /// Field identifier (None for tuple struct fields).
    pub ident: Option<syn::Ident>,

    /// Field type.
    pub ty: syn::Type,

    /// `#[my_trait(rename = "x")]`
    #[darling(default)]
    pub rename: Option<String>,

    /// `#[my_trait(skip)]` — omit this field from generation.
    #[darling(default)]
    pub skip: bool,

    /// `#[my_trait(default)]` — use Default::default() for this field.
    #[darling(default)]
    pub with_default: util::Flag,
}

/// Per-variant attributes (for enums).
#[derive(Debug, FromVariant)]
#[darling(attributes(my_trait), forward_attrs(allow, doc, cfg))]
pub struct VariantArgs {
    pub ident: syn::Ident,
    pub fields: ast::Fields<FieldArgs>,

    #[darling(default)]
    pub rename: Option<String>,

    #[darling(default)]
    pub skip: bool,
}

// ---------------------------------------------------------------------------
// 2. Manual syn::Parse  — for bespoke / unusual syntax
// ---------------------------------------------------------------------------
//
// Use this when your macro accepts syntax that doesn't look like
//   key = value, key = value, …
// For example:
//   my_macro!(Foo => bar, Baz => qux)

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Token,
};

/// Parses:  `Ident => Ident`
pub struct Mapping {
    pub from: syn::Ident,
    _arrow: Token![=>],
    pub to: syn::Ident,
}

impl Parse for Mapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            from: input.parse()?,
            _arrow: input.parse()?,
            to: input.parse()?,
        })
    }
}

/// Parses a comma-separated list of mappings:
///   `Foo => bar, Baz => qux`
pub struct MappingList {
    pub mappings: Punctuated<Mapping, Token![,]>,
}

impl Parse for MappingList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            mappings: Punctuated::parse_terminated(input)?,
        })
    }
}

// ---------------------------------------------------------------------------
// 2b. Parsing key=value attribute args without darling
//     (useful for #[proc_macro_attribute] where you only have a TokenStream)
// ---------------------------------------------------------------------------

use syn::meta::ParseNestedMeta;

#[derive(Default, Debug)]
pub struct SimpleAttrArgs {
    pub rename: Option<syn::LitStr>,
    pub version: Option<syn::LitInt>,
    pub skip: bool,
}

impl SimpleAttrArgs {
    /// Call as:
    ///   let args = SimpleAttrArgs::parse_attr(&meta)?;
    pub fn parse_nested(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        if meta.path.is_ident("rename") {
            self.rename = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("version") {
            self.version = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("skip") {
            self.skip = true;
        } else {
            return Err(meta.error("unknown attribute argument"));
        }
        Ok(())
    }
}

// Usage example (in your attribute macro expand fn):
//
//   let mut args = SimpleAttrArgs::default();
//   let attr_parser = syn::meta::parser(|meta| args.parse_nested(meta));
//   syn::parse_macro_input!(attr as syn::meta::ParseNestedMeta); // ← won't compile as-is
//
// Correct pattern:
//
//   let mut args = SimpleAttrArgs::default();
//   parse_macro_input!(attr_tokens with syn::meta::parser(|meta| args.parse_nested(meta)));

// ---------------------------------------------------------------------------
// 3. Deluxe — alternative derive-based approach
// ---------------------------------------------------------------------------
//
// Deluxe has a cleaner syntax for optional/required fields and is sometimes
// easier to compose.  Use either darling or deluxe, not both, on the same struct.

#[derive(deluxe::ExtractAttributes, Debug)]
#[deluxe(attributes(my_trait))]
pub struct DeluxeContainerArgs {
    #[deluxe(default)]
    pub rename: Option<String>,

    #[deluxe(default = true)]
    pub generate_display: bool,
}

#[derive(deluxe::ExtractAttributes, Debug)]
#[deluxe(attributes(my_trait))]
pub struct DeluxeFieldArgs {
    #[deluxe(default)]
    pub rename: Option<String>,
    #[deluxe(default)]
    pub skip: bool,
}

// ---------------------------------------------------------------------------
// Utility — iterate fields from a darling Data<V, F>
// ---------------------------------------------------------------------------

/// Returns an iterator over named fields from darling's `ast::Data`.
/// Panics if called on an enum without first matching the variant.
pub fn named_fields(
    data: &ast::Data<VariantArgs, FieldArgs>,
) -> impl Iterator<Item = &FieldArgs> {
    match data {
        ast::Data::Struct(fields) => fields.iter(),
        ast::Data::Enum(_) => panic!("called named_fields on an enum"),
    }
}
