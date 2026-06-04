use proc_macro_error2::abort;
use syn::{Ident, Pat, PatType};

pub struct PathParam {
    pub path_position: usize,
    // without the `:` from path arg
    pub name: Ident,
    #[allow(unused)]
    pub fn_position: usize,
    pub fn_type: PatType,
}

impl PathParam {
    /// warning: doesnt verify if the path param actually exists for the route
    pub fn from_pat_type(
        fn_position: usize,
        pat_type: &PatType,
        path_params: &[(usize, String)],
    ) -> Self {
        let (path_position, name) = if let Pat::Ident(ident) = &*pat_type.pat {
            let name = ident.ident.clone();
            let path_position = path_params
                .iter()
                .find(|(_, n)| n.as_str() == name.to_string().as_str())
                .map(|(p, _)| p)
                .unwrap_or_else(|| abort!(pat_type.pat, "Failed to find path param for ident"));
            (*path_position, name)
        } else {
            abort!(pat_type.pat, "expected an ident path param");
        };

        PathParam {
            path_position,
            name,
            fn_position,
            fn_type: pat_type.clone(),
        }
    }
}
