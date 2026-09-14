// wdh/src/service/encoding.rs
//
//! The encodings of the string data of the `Write` service.
use crate::error::HostError;

pub const ENCODING_ASCII: u8 = 0;

pub const ENCODING_UTF8: u8 = 1;

pub const ENCODING_UTF16: u8 = 2;

/// The string encodings of the `Write` service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// 7-bit ASCII.
    ASCII,
    UTF8,
    UTF16,
}

impl Encoding {
    /// Decodes the encoding byte of the arguments block.
    pub fn from_byte(byte: u8) -> Result<Self, HostError> {
        match byte {
            ENCODING_ASCII => Ok(Self::ASCII),
            ENCODING_UTF8 => Ok(Self::UTF8),
            ENCODING_UTF16 => Ok(Self::UTF16),
            _ => Err(HostError::UnknownEncoding { got: byte }),
        }
    }

    /// Converts the string bytes to UTF-8, validating them.
    ///
    /// ASCII and UTF-8 are written to the output as-is; UTF-16 is
    /// converted from UTF-16 LE. The validation is where the declared
    /// encoding takes effect.
    pub fn to_utf8(self, data: &[u8]) -> Result<Vec<u8>, HostError> {
        match self {
            Self::ASCII => match data.iter().position(|b| *b >= 0x80) {
                Some(offset) => Err(HostError::InvalidASCII { offset }),
                None => Ok(data.to_vec()),
            },
            Self::UTF8 => match std::str::from_utf8(data) {
                Ok(_) => Ok(data.to_vec()),
                Err(e) => Err(HostError::InvalidUTF8 {
                    offset: e.valid_up_to(),
                }),
            },
            Self::UTF16 => utf16_to_utf8(data),
        }
    }
}

fn utf16_to_utf8(data: &[u8]) -> Result<Vec<u8>, HostError> {
    if !data.len().is_multiple_of(2) {
        return Err(HostError::InvalidUTF16 { offset: data.len() });
    }

    let units: Vec<u16> = data
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();

    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        if (0xD800..=0xDBFF).contains(&unit) {
            let has_low = units
                .get(i + 1)
                .is_some_and(|next| (0xDC00..=0xDFFF).contains(next));
            if !has_low {
                return Err(HostError::InvalidUTF16 { offset: i * 2 });
            }
            i += 2;
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            return Err(HostError::InvalidUTF16 { offset: i * 2 });
        } else {
            i += 1;
        }
    }

    let mut out = String::new();
    for ch in std::char::decode_utf16(units).map(|r| r.expect("the surrogates are validated above"))
    {
        out.push(ch);
    }
    Ok(out.into_bytes())
}
