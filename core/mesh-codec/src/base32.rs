//! Canonical RFC 4648 lowercase Base32 without padding for BP endpoint IDs.

use core::fmt;

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Base32Error {
    InvalidLength,
    InvalidCharacter,
    NonCanonicalTrailingBits,
}

impl fmt::Display for Base32Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid canonical Base32: {self:?}")
    }
}

impl std::error::Error for Base32Error {}

#[must_use]
pub fn encode_16(bytes: &[u8; 16]) -> String {
    let mut output = String::with_capacity(26);
    let mut accumulator = 0u32;
    let mut bits = 0u8;
    for byte in bytes {
        accumulator = (accumulator << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((accumulator >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits != 0 {
        output.push(ALPHABET[((accumulator << (5 - bits)) & 0x1f) as usize] as char);
    }
    output
}

pub fn decode_16(text: &str) -> Result<[u8; 16], Base32Error> {
    if text.len() != 26 {
        return Err(Base32Error::InvalidLength);
    }
    let mut output = [0u8; 16];
    let mut output_index = 0usize;
    let mut accumulator = 0u32;
    let mut bits = 0u8;
    for byte in text.bytes() {
        let value = match byte {
            b'a'..=b'z' => byte - b'a',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => return Err(Base32Error::InvalidCharacter),
        };
        accumulator = (accumulator << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            if output_index >= output.len() {
                return Err(Base32Error::InvalidLength);
            }
            output[output_index] = (accumulator >> bits) as u8;
            output_index += 1;
        }
    }
    if output_index != output.len() {
        return Err(Base32Error::InvalidLength);
    }
    if bits != 0 && accumulator & ((1u32 << bits) - 1) != 0 {
        return Err(Base32Error::NonCanonicalTrailingBits);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_round_trip() {
        let bytes = *b"0123456789abcdef";
        let encoded = encode_16(&bytes);
        assert_eq!(encoded.len(), 26);
        assert_eq!(decode_16(&encoded).unwrap(), bytes);
    }

    #[test]
    fn rejects_uppercase_padding_and_trailing_bits() {
        assert_eq!(
            decode_16("A".repeat(26).as_str()),
            Err(Base32Error::InvalidCharacter)
        );
        assert_eq!(
            decode_16("a".repeat(25).as_str()),
            Err(Base32Error::InvalidLength)
        );
        assert_eq!(
            decode_16("aaaaaaaaaaaaaaaaaaaaaaaaab"),
            Err(Base32Error::NonCanonicalTrailingBits)
        );
    }
}
