//! RFC 8949 Core Deterministic CBOR for the constrained protocol surface.

use core::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CborValue {
    Unsigned(u64),
    Negative(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<CborValue>),
    Map(Vec<(CborValue, CborValue)>),
    Bool(bool),
    Null,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeLimits {
    pub max_input_bytes: usize,
    pub max_byte_string_bytes: usize,
    pub max_text_bytes: usize,
    pub max_collection_items: usize,
    pub max_depth: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 12_288,
            max_byte_string_bytes: 8_192,
            max_text_bytes: 7_800,
            max_collection_items: 32,
            max_depth: 12,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CborError {
    Empty,
    Truncated,
    TrailingData,
    InputTooLarge,
    UnsupportedType,
    IndefiniteLength,
    NonCanonical,
    InvalidUtf8,
    LengthExceeded,
    DepthExceeded,
    DuplicateMapKey,
    MapKeyOrder,
    IntegerOutOfRange,
}

impl fmt::Display for CborError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid deterministic CBOR: {self:?}")
    }
}

impl std::error::Error for CborError {}

pub fn encode_deterministic(value: &CborValue) -> Result<Vec<u8>, CborError> {
    let mut output = Vec::new();
    encode_value(value, &mut output)?;
    Ok(output)
}

pub fn decode_deterministic(input: &[u8], limits: DecodeLimits) -> Result<CborValue, CborError> {
    let (value, consumed) = decode_deterministic_prefix(input, limits)?;
    if consumed != input.len() {
        return Err(CborError::TrailingData);
    }
    Ok(value)
}

pub fn decode_deterministic_prefix(
    input: &[u8],
    limits: DecodeLimits,
) -> Result<(CborValue, usize), CborError> {
    if input.is_empty() {
        return Err(CborError::Empty);
    }
    if input.len() > limits.max_input_bytes {
        return Err(CborError::InputTooLarge);
    }
    let mut decoder = Decoder {
        input,
        position: 0,
        limits,
    };
    let value = decoder.parse(0)?;
    Ok((value, decoder.position))
}

fn encode_value(value: &CborValue, output: &mut Vec<u8>) -> Result<(), CborError> {
    match value {
        CborValue::Unsigned(value) => encode_argument(0, *value, output),
        CborValue::Negative(value) => {
            if *value >= 0 {
                return Err(CborError::IntegerOutOfRange);
            }
            let encoded = u64::try_from(-1i128 - i128::from(*value))
                .map_err(|_| CborError::IntegerOutOfRange)?;
            encode_argument(1, encoded, output);
        }
        CborValue::Bytes(bytes) => {
            encode_argument(2, bytes.len() as u64, output);
            output.extend_from_slice(bytes);
        }
        CborValue::Text(text) => {
            encode_argument(3, text.len() as u64, output);
            output.extend_from_slice(text.as_bytes());
        }
        CborValue::Array(values) => {
            encode_argument(4, values.len() as u64, output);
            for value in values {
                encode_value(value, output)?;
            }
        }
        CborValue::Map(entries) => {
            let mut encoded = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let key = encode_deterministic(key)?;
                let value = encode_deterministic(value)?;
                encoded.push((key, value));
            }
            encoded.sort_by(|left, right| canonical_key_cmp(&left.0, &right.0));
            if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(CborError::DuplicateMapKey);
            }
            encode_argument(5, encoded.len() as u64, output);
            for (key, value) in encoded {
                output.extend_from_slice(&key);
                output.extend_from_slice(&value);
            }
        }
        CborValue::Bool(false) => output.push(0xf4),
        CborValue::Bool(true) => output.push(0xf5),
        CborValue::Null => output.push(0xf6),
    }
    Ok(())
}

fn encode_argument(major: u8, value: u64, output: &mut Vec<u8>) {
    let prefix = major << 5;
    match value {
        0..=23 => output.push(prefix | value as u8),
        24..=0xff => output.extend_from_slice(&[prefix | 24, value as u8]),
        0x100..=0xffff => {
            output.push(prefix | 25);
            output.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            output.push(prefix | 26);
            output.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            output.push(prefix | 27);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn canonical_key_cmp(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

struct Decoder<'a> {
    input: &'a [u8],
    position: usize,
    limits: DecodeLimits,
}

impl Decoder<'_> {
    fn parse(&mut self, depth: usize) -> Result<CborValue, CborError> {
        if depth > self.limits.max_depth {
            return Err(CborError::DepthExceeded);
        }
        let initial = self.take_byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        match major {
            0 => Ok(CborValue::Unsigned(self.argument(additional)?)),
            1 => {
                let encoded = self.argument(additional)?;
                let value = -1i128 - i128::from(encoded);
                let value = i64::try_from(value).map_err(|_| CborError::IntegerOutOfRange)?;
                Ok(CborValue::Negative(value))
            }
            2 => {
                let length = self.length(additional, self.limits.max_byte_string_bytes)?;
                Ok(CborValue::Bytes(self.take(length)?.to_vec()))
            }
            3 => {
                let length = self.length(additional, self.limits.max_text_bytes)?;
                let text =
                    core::str::from_utf8(self.take(length)?).map_err(|_| CborError::InvalidUtf8)?;
                Ok(CborValue::Text(text.to_owned()))
            }
            4 => {
                let length = self.length(additional, self.limits.max_collection_items)?;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(self.parse(depth + 1)?);
                }
                Ok(CborValue::Array(values))
            }
            5 => {
                let length = self.length(additional, self.limits.max_collection_items)?;
                let mut entries = Vec::with_capacity(length);
                let mut previous_key: Option<&[u8]> = None;
                for _ in 0..length {
                    let start = self.position;
                    let key = self.parse(depth + 1)?;
                    let end = self.position;
                    let raw_key = &self.input[start..end];
                    if let Some(previous) = previous_key {
                        match canonical_key_cmp(previous, raw_key) {
                            core::cmp::Ordering::Less => {}
                            core::cmp::Ordering::Equal => return Err(CborError::DuplicateMapKey),
                            core::cmp::Ordering::Greater => return Err(CborError::MapKeyOrder),
                        }
                    }
                    previous_key = Some(raw_key);
                    let value = self.parse(depth + 1)?;
                    entries.push((key, value));
                }
                Ok(CborValue::Map(entries))
            }
            7 => match additional {
                20 => Ok(CborValue::Bool(false)),
                21 => Ok(CborValue::Bool(true)),
                22 => Ok(CborValue::Null),
                31 => Err(CborError::IndefiniteLength),
                _ => Err(CborError::UnsupportedType),
            },
            _ => Err(CborError::UnsupportedType),
        }
    }

    fn length(&mut self, additional: u8, maximum: usize) -> Result<usize, CborError> {
        let value = self.argument(additional)?;
        let value = usize::try_from(value).map_err(|_| CborError::LengthExceeded)?;
        if value > maximum {
            return Err(CborError::LengthExceeded);
        }
        Ok(value)
    }

    fn argument(&mut self, additional: u8) -> Result<u64, CborError> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => {
                let value = u64::from(self.take_byte()?);
                if value < 24 {
                    return Err(CborError::NonCanonical);
                }
                Ok(value)
            }
            25 => {
                let value = u64::from(u16::from_be_bytes(
                    self.take(2)?.try_into().expect("fixed slice"),
                ));
                if value <= 0xff {
                    return Err(CborError::NonCanonical);
                }
                Ok(value)
            }
            26 => {
                let value = u64::from(u32::from_be_bytes(
                    self.take(4)?.try_into().expect("fixed slice"),
                ));
                if value <= 0xffff {
                    return Err(CborError::NonCanonical);
                }
                Ok(value)
            }
            27 => {
                let value = u64::from_be_bytes(self.take(8)?.try_into().expect("fixed slice"));
                if value <= 0xffff_ffff {
                    return Err(CborError::NonCanonical);
                }
                Ok(value)
            }
            31 => Err(CborError::IndefiniteLength),
            _ => Err(CborError::UnsupportedType),
        }
    }

    fn take_byte(&mut self) -> Result<u8, CborError> {
        let byte = *self.input.get(self.position).ok_or(CborError::Truncated)?;
        self.position += 1;
        Ok(byte)
    }

    fn take(&mut self, length: usize) -> Result<&[u8], CborError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(CborError::LengthExceeded)?;
        let bytes = self
            .input
            .get(self.position..end)
            .ok_or(CborError::Truncated)?;
        self.position = end;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn canonical_boundaries_match_rfc_8949() {
        for (value, bytes) in [
            (23, vec![0x17]),
            (24, vec![0x18, 0x18]),
            (255, vec![0x18, 0xff]),
            (256, vec![0x19, 0x01, 0x00]),
            (65_536, vec![0x1a, 0x00, 0x01, 0x00, 0x00]),
        ] {
            assert_eq!(
                encode_deterministic(&CborValue::Unsigned(value)).unwrap(),
                bytes
            );
            assert_eq!(
                decode_deterministic(&bytes, DecodeLimits::default()).unwrap(),
                CborValue::Unsigned(value)
            );
        }
    }

    #[test]
    fn rejects_noncanonical_and_indefinite_values() {
        assert_eq!(
            decode_deterministic(&[0x18, 0x17], DecodeLimits::default()),
            Err(CborError::NonCanonical)
        );
        assert_eq!(
            decode_deterministic(&[0x9f, 0xff], DecodeLimits::default()),
            Err(CborError::IndefiniteLength)
        );
    }

    #[test]
    fn canonical_map_order_is_enforced() {
        let noncanonical = [0xa2, 0x02, 0x00, 0x01, 0x00];
        assert_eq!(
            decode_deterministic(&noncanonical, DecodeLimits::default()),
            Err(CborError::MapKeyOrder)
        );
    }

    proptest! {
        #[test]
        fn unsigned_round_trip_is_canonical(value in any::<u64>()) {
            let value = CborValue::Unsigned(value);
            let encoded = encode_deterministic(&value).unwrap();
            let decoded = decode_deterministic(&encoded, DecodeLimits::default()).unwrap();
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn arbitrary_input_never_panics(input in prop::collection::vec(any::<u8>(), 0..20_000)) {
            let _ = decode_deterministic(&input, DecodeLimits::default());
        }
    }
}
