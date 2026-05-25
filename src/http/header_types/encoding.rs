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
