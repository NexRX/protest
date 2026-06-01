use darling::FromMeta;
use syn::Ident;

/// Parsed form of `#[service(method = GET, path = "/hello")]` on a method.
///
/// Darling handles all parsing, validation, and error reporting automatically.
/// The `suggestions` feature will even suggest correct field names on typos.
#[derive(Debug, FromMeta)]
pub struct RouteArgs {
    /// HTTP verb, e.g. `GET`, `POST`, `PUT`, `DELETE`
    pub method: Ident,
    /// Route path, e.g. `"/hello"`
    pub path: String,
}
