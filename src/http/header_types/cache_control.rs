use std::{fmt, str::FromStr, time::Duration};
use strum::Display;

/// The HTTP `Cache-Control` header holds directives (instructions) in both requests and
/// responses that control caching in browsers and shared caches (e.g. Proxies, CDNs).
///
/// Multiple directives are comma-separated when serialized (e.g. `max-age=604800, immutable`).
///
/// See [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control).
#[derive(Debug, Clone, Default)]
pub struct CacheControl {
    /// Indicates whether the response may be cached by shared caches ([`Cachability::Public`]),
    /// only private caches ([`Cachability::Private`]), or must be revalidated before each reuse
    /// ([`Cachability::NoCache`]).
    pub cachability: Option<Cachability>,

    /// The `max-age=N` response directive indicates that the response remains fresh until N
    /// seconds after the response is generated.
    ///
    /// Note that `max-age` is not the elapsed time since the response was received; it is the
    /// elapsed time since the response was generated on the origin server.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#max-age).
    pub max_age: Option<Duration>,

    /// The `s-maxage` response directive indicates how long the response remains fresh in a
    /// shared cache. The `s-maxage` directive is ignored by private caches, and overrides the
    /// value specified by the `max-age` directive or the `Expires` header for shared caches, if
    /// they are present.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#s-maxage).
    pub s_maxage: Option<Duration>,

    /// The `no-store` response directive indicates that any caches of any kind (private or
    /// shared) should not store this response.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#no-store).
    pub no_store: bool,

    /// The `no-transform` directive indicates that any intermediary (regardless of whether it
    /// implements a cache) shouldn't transform the response contents.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#no-transform).
    pub no_transform: bool,

    /// The `must-revalidate` response directive indicates that the response can be stored in
    /// caches and can be reused while fresh. If the response becomes stale, it must be validated
    /// with the origin server before reuse.
    ///
    /// Typically used with `max-age`. Prevents caches from serving stale responses when
    /// disconnected from the origin server (which HTTP otherwise permits).
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#must-revalidate).
    pub must_revalidate: bool,

    /// The `proxy-revalidate` response directive is the equivalent of `must-revalidate`, but
    /// specifically for shared caches only.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#proxy-revalidate).
    pub proxy_revalidate: bool,

    /// The `must-understand` response directive indicates that a cache should store the response
    /// only if it understands the requirements for caching based on status code.
    ///
    /// Should be coupled with `no-store` for fallback behavior: if a cache doesn't support
    /// `must-understand`, it falls back to `no-store`.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#must-understand).
    pub must_understand: bool,

    /// The `immutable` response directive indicates that the response will not be updated while
    /// it's fresh. Used alongside long `max-age` values and cache-busting URL patterns to avoid
    /// unnecessary conditional requests on reload.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#immutable).
    pub immutable: bool,

    /// The `stale-while-revalidate` response directive indicates that the cache may reuse a
    /// stale response for the given number of seconds while it revalidates in the background.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#stale-while-revalidate).
    pub stale_while_revalidate: Option<Duration>,

    /// The `stale-if-error` response directive indicates that the cache can reuse a stale
    /// response for the given number of seconds when an upstream server generates an error
    /// (status 500, 502, 503, or 504).
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#stale-if-error).
    pub stale_if_error: Option<Duration>,
}

impl fmt::Display for CacheControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();

        if let Some(cachability) = &self.cachability {
            parts.push(cachability.to_string());
        }
        if let Some(max_age) = self.max_age {
            parts.push(format!("max-age={}", max_age.as_secs()));
        }
        if let Some(s_maxage) = self.s_maxage {
            parts.push(format!("s-maxage={}", s_maxage.as_secs()));
        }
        if self.no_store {
            parts.push("no-store".to_string());
        }
        if self.no_transform {
            parts.push("no-transform".to_string());
        }
        if self.must_revalidate {
            parts.push("must-revalidate".to_string());
        }
        if self.proxy_revalidate {
            parts.push("proxy-revalidate".to_string());
        }
        if self.must_understand {
            parts.push("must-understand".to_string());
        }
        if self.immutable {
            parts.push("immutable".to_string());
        }
        if let Some(swr) = self.stale_while_revalidate {
            parts.push(format!("stale-while-revalidate={}", swr.as_secs()));
        }
        if let Some(sie) = self.stale_if_error {
            parts.push(format!("stale-if-error={}", sie.as_secs()));
        }

        write!(f, "{}", parts.join(", "))
    }
}

/// Error returned when a `Cache-Control` header value cannot be parsed.
#[derive(Debug)]
pub struct ParseCacheControlError(String);

impl fmt::Display for ParseCacheControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Cache-Control directive: {}", self.0)
    }
}

impl std::error::Error for ParseCacheControlError {}

impl FromStr for CacheControl {
    type Err = ParseCacheControlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut cc = CacheControl::default();

        for raw in s.split(',') {
            let directive = raw.trim();
            if directive.is_empty() {
                continue;
            }

            let (key, value) = match directive.split_once('=') {
                Some((k, v)) => (k.trim(), Some(v.trim())),
                None => (directive, None),
            };

            let parse_secs = |v: Option<&str>| -> Result<Duration, ParseCacheControlError> {
                let v =
                    v.ok_or_else(|| ParseCacheControlError(format!("'{key}' requires a value")))?;
                v.parse::<u64>().map(Duration::from_secs).map_err(|_| {
                    ParseCacheControlError(format!("'{key}={v}' is not a valid integer"))
                })
            };

            match key {
                "public" => cc.cachability = Some(Cachability::Public),
                "private" => cc.cachability = Some(Cachability::Private),
                "no-cache" => cc.cachability = Some(Cachability::NoCache),
                "max-age" => cc.max_age = Some(parse_secs(value)?),
                "s-maxage" => cc.s_maxage = Some(parse_secs(value)?),
                "no-store" => cc.no_store = true,
                "no-transform" => cc.no_transform = true,
                "must-revalidate" => cc.must_revalidate = true,
                "proxy-revalidate" => cc.proxy_revalidate = true,
                "must-understand" => cc.must_understand = true,
                "immutable" => cc.immutable = true,
                "stale-while-revalidate" => cc.stale_while_revalidate = Some(parse_secs(value)?),
                "stale-if-error" => cc.stale_if_error = Some(parse_secs(value)?),
                _ => {} // Unknown directives are ignored
            }
        }

        Ok(cc)
    }
}

/// Indicates the basic cacheability of a response.
///
/// See [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Cachability {
    /// The response can be stored in a shared cache. Responses for requests with an
    /// `Authorization` header are normally not shared-cacheable, but `public` overrides that
    /// restriction.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#public).
    Public,

    /// The response can be stored only in a private cache (e.g. the browser cache). Should be
    /// used for user-personalized content, such as responses received after login or session data
    /// managed via cookies.
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#private).
    Private,

    /// The response can be stored in caches, but must be validated with the origin server before
    /// each reuse, even when the cache is disconnected from the origin server.
    ///
    /// Note: `no-cache` does not mean "don't cache" — it means "revalidate before reuse". To
    /// prevent storage entirely, use [`CacheControl::no_store`].
    ///
    /// See [MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control#no-cache).
    NoCache,
}
