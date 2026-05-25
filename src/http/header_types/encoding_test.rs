use crate::ParseHeaderError;

use super::*;
use quiche::h3;

fn make_header(value: &[u8]) -> h3::Header {
    h3::Header::new(b"accept-encoding", value)
}

#[test]
fn parse_gzip() {
    let encodings = Encoding::try_from_header(&make_header(b"gzip")).unwrap();
    assert_eq!(encodings.len(), 1);
    assert!(matches!(encodings[0], Encoding::Gzip(None)));
}

#[test]
fn parse_br_with_weight() {
    let encodings = Encoding::try_from_header(&make_header(b"br;q=0.9")).unwrap();
    assert_eq!(encodings.len(), 1);
    match encodings[0] {
        Encoding::Br(Some(q)) => assert!((q - 0.9_f32).abs() < 1e-5),
        ref other => panic!("expected Br(Some(≈0.9)), got {other:?}"),
    }
}

#[test]
fn parse_multiple_encodings() {
    let encodings = Encoding::try_from_header(&make_header(b"gzip deflate br")).unwrap();
    assert_eq!(encodings.len(), 3);
    assert!(matches!(encodings[0], Encoding::Gzip(None)));
    assert!(matches!(encodings[1], Encoding::Deflate(None)));
    assert!(matches!(encodings[2], Encoding::Br(None)));
}

#[test]
fn parse_wildcard() {
    let encodings = Encoding::try_from_header(&make_header(b"*")).unwrap();
    assert_eq!(encodings.len(), 1);
    assert!(matches!(encodings[0], Encoding::Wildcard(None)));
}

#[test]
fn parse_unknown_encoding() {
    let encodings = Encoding::try_from_header(&make_header(b"foobar")).unwrap();
    assert_eq!(encodings.len(), 1);
    assert!(matches!(encodings[0], Encoding::Unknown(None)));
}

#[test]
fn parse_invalid_utf8_returns_err() {
    let result = Encoding::try_from_header(&make_header(&[0xFF, 0xFE]));
    assert!(matches!(result, Err(ParseHeaderError::BadValue(_, _))));
}

#[test]
fn weight_none_for_unweighted() {
    assert_eq!(Encoding::Gzip(None).weight(), None);
}

#[test]
fn weight_some_for_weighted() {
    assert_eq!(Encoding::Br(Some(0.5)).weight(), Some(0.5));
}

#[test]
fn to_string_with_weight_no_weight() {
    assert_eq!(Encoding::Gzip(None).to_string_with_weight(), "gzip");
}

#[test]
fn to_string_with_weight_with_weight() {
    assert_eq!(Encoding::Br(Some(0.8)).to_string_with_weight(), "br;q=0.8");
}

#[test]
fn display_all_variants() {
    assert_eq!(Encoding::Gzip(None).to_string(), "gzip");
    assert_eq!(Encoding::Compress(None).to_string(), "compress");
    assert_eq!(Encoding::Deflate(None).to_string(), "deflate");
    assert_eq!(Encoding::Br(None).to_string(), "br");
    assert_eq!(Encoding::Zstd(None).to_string(), "zstd");
    assert_eq!(Encoding::Dcb(None).to_string(), "dcb");
    assert_eq!(Encoding::Dcz(None).to_string(), "dcz");
    assert_eq!(Encoding::Identity(None).to_string(), "identity");
    assert_eq!(Encoding::Wildcard(None).to_string(), "*");
    assert_eq!(Encoding::Unknown(None).to_string(), "unknown");
}

#[test]
fn default_is_identity_none() {
    assert!(matches!(Encoding::default(), Encoding::Identity(None)));
}

#[test]
fn is_variant_helpers() {
    assert!(Encoding::Gzip(None).is_gzip());
    assert!(!Encoding::Gzip(None).is_identity());
    assert!(Encoding::Identity(None).is_identity());
    assert!(Encoding::Wildcard(None).is_wildcard());
}
