use super::cache_control::*;
use std::time::Duration;

#[test]
fn parse_cachability_public() {
    let cc: CacheControl = "public".parse().unwrap();
    assert_eq!(cc.cachability, Some(Cachability::Public));
}

#[test]
fn parse_cachability_private() {
    let cc: CacheControl = "private".parse().unwrap();
    assert_eq!(cc.cachability, Some(Cachability::Private));
}

#[test]
fn parse_cachability_no_cache() {
    let cc: CacheControl = "no-cache".parse().unwrap();
    assert_eq!(cc.cachability, Some(Cachability::NoCache));
}

#[test]
fn parse_max_age() {
    let cc: CacheControl = "max-age=3600".parse().unwrap();
    assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
}

#[test]
fn parse_s_maxage() {
    let cc: CacheControl = "s-maxage=86400".parse().unwrap();
    assert_eq!(cc.s_maxage, Some(Duration::from_secs(86400)));
}

#[test]
fn parse_stale_while_revalidate() {
    let cc: CacheControl = "stale-while-revalidate=60".parse().unwrap();
    assert_eq!(cc.stale_while_revalidate, Some(Duration::from_secs(60)));
}

#[test]
fn parse_stale_if_error() {
    let cc: CacheControl = "stale-if-error=3600".parse().unwrap();
    assert_eq!(cc.stale_if_error, Some(Duration::from_secs(3600)));
}

#[test]
fn parse_boolean_flags() {
    let cc: CacheControl =
        "no-store, no-transform, must-revalidate, proxy-revalidate, must-understand, immutable"
            .parse()
            .unwrap();
    assert!(cc.no_store);
    assert!(cc.no_transform);
    assert!(cc.must_revalidate);
    assert!(cc.proxy_revalidate);
    assert!(cc.must_understand);
    assert!(cc.immutable);
}

#[test]
fn parse_combined() {
    let cc: CacheControl = "public, max-age=604800, immutable".parse().unwrap();
    assert_eq!(cc.cachability, Some(Cachability::Public));
    assert_eq!(cc.max_age, Some(Duration::from_secs(604800)));
    assert!(cc.immutable);
}

#[test]
fn display_roundtrip_contains_all_parts() {
    let s = "public, max-age=604800, immutable"
        .parse::<CacheControl>()
        .unwrap()
        .to_string();
    assert!(s.contains("public"));
    assert!(s.contains("max-age=604800"));
    assert!(s.contains("immutable"));
}

#[test]
fn unknown_directive_is_ignored() {
    let cc: CacheControl = "foo-bar=123".parse().unwrap();
    assert!(cc.cachability.is_none());
    assert!(cc.max_age.is_none());
    assert!(cc.s_maxage.is_none());
    assert!(cc.stale_while_revalidate.is_none());
    assert!(cc.stale_if_error.is_none());
    assert!(!cc.no_store);
    assert!(!cc.no_transform);
    assert!(!cc.must_revalidate);
    assert!(!cc.proxy_revalidate);
    assert!(!cc.must_understand);
    assert!(!cc.immutable);
}

#[test]
fn error_max_age_non_integer() {
    assert!("max-age=abc".parse::<CacheControl>().is_err());
}

#[test]
fn error_max_age_missing_value() {
    assert!("max-age".parse::<CacheControl>().is_err());
}

#[test]
fn cachability_display() {
    assert_eq!(Cachability::Public.to_string(), "public");
    assert_eq!(Cachability::Private.to_string(), "private");
    assert_eq!(Cachability::NoCache.to_string(), "no-cache");
}
