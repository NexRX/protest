use crate::{ACCEPT_ENCODING, ParseHeaderError};
use quiche::h3::{self, NameValue as _};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, derive_more::IsVariant, serde::Deserialize)]
pub enum Encoding {
    /// A compression format that uses the Lempel-Ziv coding (LZ77) with a 32-bit CRC.
    Gzip(Option<f32>),
    /// A compression format that uses the Lempel-Ziv-Welch (LZW) algorithm.
    Compress(Option<f32>),
    /// A compression format that uses the zlib structure with the deflate compression algorithm.
    Deflate(Option<f32>),
    /// A compression format that uses the Brotli algorithm.
    Br(Option<f32>),
    /// A compression format that uses the Zstandard algorithm.
    Zstd(Option<f32>),
    /// A format that uses the Dictionary-Compressed Brotli algorithm. See Compression Dictionary Transport.
    Dcb(Option<f32>),
    /// A format that uses the Dictionary-Compressed Zstandard algorithm. See Compression Dictionary Transport.
    Dcz(Option<f32>),
    /// Indicates the identity function (that is, without modification or compression). This value is always considered as acceptable, even if omitted.
    Identity(Option<f32>),
    /// (\*) Matches any content encoding not already listed in the header. This is the default value if the header is not present. This directive does not suggest that any algorithm is supported but indicates that no preference is expressed.
    Wildcard(Option<f32>),
    /// A content type unknown to the server
    Unknown(Option<f32>),
}

impl Encoding {
    pub fn try_from_header(value: &h3::Header) -> Result<Vec<Self>, ParseHeaderError> {
        let mut encodings = Vec::new();

        let values = str::from_utf8(value.value())
            .map_err(|e| ParseHeaderError::BadValue(ACCEPT_ENCODING.to_string(), e))?
            .split_whitespace();

        for value in values {
            let split = value.split(";q=").collect::<Vec<_>>();
            let encoding = split[0];
            let weight = split.get(1).map_or(Ok(None), |w| {
                w.parse::<f32>()
                    .map(Some)
                    .map_err(|e| ParseHeaderError::ExpectedFloat(ACCEPT_ENCODING.to_string(), e))
            })?;

            encodings.push(match encoding {
                "gzip" => Encoding::Gzip(weight),
                "compress" => Encoding::Compress(weight),
                "deflate" => Encoding::Deflate(weight),
                "br" => Encoding::Br(weight),
                "zstd" => Encoding::Zstd(weight),
                "dcb" => Encoding::Dcb(weight),
                "dcz" => Encoding::Dcz(weight),
                "identity" => Encoding::Identity(weight),
                "*" => Encoding::Wildcard(weight),
                _ => Encoding::Unknown(weight),
            });
        }
        Ok(encodings)
    }

    pub fn weight(&self) -> Option<f32> {
        match self {
            Encoding::Gzip(weight)
            | Encoding::Compress(weight)
            | Encoding::Deflate(weight)
            | Encoding::Br(weight)
            | Encoding::Zstd(weight)
            | Encoding::Dcb(weight)
            | Encoding::Dcz(weight)
            | Encoding::Identity(weight)
            | Encoding::Wildcard(weight)
            | Encoding::Unknown(weight) => *weight,
        }
    }

    pub fn to_string_with_weight(&self) -> String {
        let encoding_str = self.to_string();
        if let Some(weight) = self.weight() {
            format!("{encoding_str};q={weight}")
        } else {
            encoding_str
        }
    }
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::Identity(None)
    }
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Encoding::Gzip(_) => "gzip",
            Encoding::Compress(_) => "compress",
            Encoding::Deflate(_) => "deflate",
            Encoding::Br(_) => "br",
            Encoding::Zstd(_) => "zstd",
            Encoding::Dcb(_) => "dcb",
            Encoding::Dcz(_) => "dcz",
            Encoding::Identity(_) => "identity",
            Encoding::Wildcard(_) => "*",
            Encoding::Unknown(_) => "unknown",
        };

        write!(f, "{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParseHeaderError;
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
}
