use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PathMatcher<'a>(Vec<PathComponent<'a>>);

impl PathMatcher<'_> {
    pub fn matches(&self, path: &str) -> bool {
        if self.0.is_empty() && path == "/" {
            return true;
        }

        let mut components = path.split("/").filter(|c| !c.is_empty());
        for component in &self.0 {
            match components.next() {
                Some(actual) if component != actual => {
                    return false;
                }
                None => {
                    return false;
                }
                _ => {}
            }
        }

        components.next().is_none()
    }
}

impl<'a> From<&'a Path> for PathMatcher<'a> {
    fn from(path: &'a Path) -> Self {
        Self(
            path.iter()
                .map(|c| PathComponent::Static(c.to_str().unwrap()))
                .collect(),
        )
    }
}

impl<'a> From<&'a PathBuf> for PathMatcher<'a> {
    fn from(path: &'a PathBuf) -> Self {
        Self::from(path.as_path())
    }
}

impl<'a> From<&'a str> for PathMatcher<'a> {
    fn from(path: &'a str) -> Self {
        Self(
            path.split('/')
                .filter(|c| !c.is_empty())
                .map(|c| match c {
                    "*" => PathComponent::Wildcard,
                    _ if c.starts_with(':') => PathComponent::Param(&c[1..]),
                    _ => PathComponent::Static(c),
                })
                .collect(),
        )
    }
}

impl<'a> From<&'a String> for PathMatcher<'a> {
    fn from(path: &'a String) -> Self {
        Self::from(path.as_str())
    }
}

impl<'a> From<&'a [PathComponent<'a>]> for PathMatcher<'a> {
    fn from(path: &'a [PathComponent<'a>]) -> Self {
        Self(path.to_vec())
    }
}

impl<'a> From<Vec<PathComponent<'a>>> for PathMatcher<'a> {
    fn from(path: Vec<PathComponent<'a>>) -> Self {
        Self(path)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum PathComponent<'a> {
    Static(&'a str),
    Param(&'a str),
    Wildcard,
}

impl<'a> PartialEq<str> for PathComponent<'a> {
    fn eq(&self, actual: &str) -> bool {
        match self {
            PathComponent::Static(expected) => actual == *expected,
            PathComponent::Param(_) => true,
            PathComponent::Wildcard => true,
        }
    }
}

impl<'a> PartialEq<String> for PathComponent<'a> {
    fn eq(&self, actual: &String) -> bool {
        self == actual.as_str()
    }
}

impl<'a> PartialEq<Path> for PathComponent<'a> {
    fn eq(&self, actual: &Path) -> bool {
        match self {
            PathComponent::Static(expected) => actual == *expected,
            PathComponent::Param(_) => true,
            PathComponent::Wildcard => true,
        }
    }
}

impl<'a> PartialEq<PathBuf> for PathComponent<'a> {
    fn eq(&self, actual: &PathBuf) -> bool {
        match self {
            PathComponent::Static(expected) => actual == *expected,
            PathComponent::Param(_) => true,
            PathComponent::Wildcard => true,
        }
    }
}

impl<'a> PartialEq<OsStr> for PathComponent<'a> {
    fn eq(&self, actual: &OsStr) -> bool {
        match self {
            PathComponent::Static(expected) => actual == *expected,
            PathComponent::Param(_) => true,
            PathComponent::Wildcard => true,
        }
    }
}

#[macro_export]
macro_rules! path_matcher {
    (@component *) => {
        $crate::PathComponent::Wildcard
    };
    (@component : $name:ident) => {
        $crate::PathComponent::Param(stringify!($name))
    };
    (@component $literal:literal) => {
        $crate::PathComponent::Static($literal)
    };
    ($single:literal) => {
        $crate::PathMatcher::from($single)
    };
    // Token-munching arms to handle `:ident` (two tokens) in comma-separated lists
    (@munch [$($acc:expr),*]) => {
        $crate::PathMatcher::from(vec![$($acc),*])
    };
    (@munch [$($acc:expr),*] ,) => {
        $crate::PathMatcher::from(vec![$($acc),*])
    };
    (@munch [$($acc:expr),*] , : $name:ident $($rest:tt)*) => {
        path_matcher!(@munch [$($acc,)* path_matcher!(@component : $name)] $($rest)*)
    };
    (@munch [$($acc:expr),*] , $component:tt $($rest:tt)*) => {
        path_matcher!(@munch [$($acc,)* path_matcher!(@component $component)] $($rest)*)
    };
    (: $name:ident $($rest:tt)*) => {
        path_matcher!(@munch [path_matcher!(@component : $name)] $($rest)*)
    };
    ($first:tt $($rest:tt)*) => {
        path_matcher!(@munch [path_matcher!(@component $first)] $($rest)*)
    };
}

#[cfg(test)]
mod tests {
    use crate::{PathComponent, PathMatcher};
    use std::path::Path;

    #[test]
    fn test_path_matcher() {
        assert_eq!(
            path_matcher!("/foo/*/bar"),
            PathMatcher::from(vec![
                PathComponent::Static("foo"),
                PathComponent::Wildcard,
                PathComponent::Static("bar"),
            ]),
        );
    }

    #[test]
    fn test_path_matcher_multi_token() {
        assert_eq!(
            path_matcher!("foo", *, "bar"),
            PathMatcher::from(vec![
                PathComponent::Static("foo"),
                PathComponent::Wildcard,
                PathComponent::Static("bar"),
            ]),
        );
    }

    #[test]
    fn test_path_matcher_static_only() {
        assert_eq!(
            path_matcher!("/api/v1/users"),
            PathMatcher::from(vec![
                PathComponent::Static("api"),
                PathComponent::Static("v1"),
                PathComponent::Static("users"),
            ]),
        );
    }

    #[test]
    fn test_path_matcher_wildcard_only() {
        assert_eq!(
            path_matcher!("/*"),
            PathMatcher::from(vec![PathComponent::Wildcard]),
        );
    }

    #[test]
    fn test_path_matcher_multiple_wildcards() {
        assert_eq!(
            path_matcher!("/*/items/*"),
            PathMatcher::from(vec![
                PathComponent::Wildcard,
                PathComponent::Static("items"),
                PathComponent::Wildcard,
            ]),
        );
    }

    #[test]
    fn test_path_matcher_trailing_comma() {
        assert_eq!(
            path_matcher!("a", "b", "c",),
            PathMatcher::from(vec![
                PathComponent::Static("a"),
                PathComponent::Static("b"),
                PathComponent::Static("c"),
            ]),
        );
    }

    #[test]
    fn test_from_slice() {
        let components = [PathComponent::Static("foo"), PathComponent::Wildcard];
        assert_eq!(
            PathMatcher::from(components.as_slice()),
            PathMatcher::from(vec![PathComponent::Static("foo"), PathComponent::Wildcard,]),
        );
    }

    #[test]
    fn test_from_path() {
        let path = Path::new("/foo/bar");
        let matcher = PathMatcher::from(path);
        // Path keeps the leading "/" as a component
        assert_eq!(
            matcher,
            PathMatcher::from(vec![
                PathComponent::Static("/"),
                PathComponent::Static("foo"),
                PathComponent::Static("bar"),
            ]),
        );
    }

    #[test]
    fn test_empty_path() {
        let matcher = PathMatcher::from("");
        assert_eq!(matcher, PathMatcher::from(vec![]));
    }

    #[test]
    fn test_param_from_str() {
        let matcher = PathMatcher::from("/users/:id/posts");
        assert_eq!(
            matcher,
            PathMatcher::from(vec![
                PathComponent::Static("users"),
                PathComponent::Param("id"),
                PathComponent::Static("posts"),
            ]),
        );
    }

    #[test]
    fn test_param_and_wildcard_mixed() {
        assert_eq!(
            path_matcher!("api", :version, *, "detail"),
            PathMatcher::from(vec![
                PathComponent::Static("api"),
                PathComponent::Param("version"),
                PathComponent::Wildcard,
                PathComponent::Static("detail"),
            ]),
        );
    }

    #[test]
    fn test_param_at_start() {
        assert_eq!(
            path_matcher!(:tenant, "users"),
            PathMatcher::from(vec![
                PathComponent::Param("tenant"),
                PathComponent::Static("users"),
            ]),
        );
    }

    #[test]
    fn test_param_at_end() {
        assert_eq!(
            path_matcher!("users", :id),
            PathMatcher::from(vec![
                PathComponent::Static("users"),
                PathComponent::Param("id"),
            ]),
        );
    }

    #[test]
    fn test_root_path_matches() {
        let matcher = PathMatcher::from("/");
        assert!(matcher.matches("/"));
    }

    #[test]
    fn test_root_path_does_not_match_subpath() {
        let matcher = PathMatcher::from("/");
        assert!(!matcher.matches("/foo"));
    }

    #[test]
    fn test_absolute_path_with_param_matches() {
        let matcher = PathMatcher::from("/user/string/:name");
        assert!(matcher.matches("/user/string/john"));
        assert!(matcher.matches("/user/string/smith"));
        assert!(!matcher.matches("/user/string"));
        assert!(!matcher.matches("/user/string/john/extra"));
    }
}
