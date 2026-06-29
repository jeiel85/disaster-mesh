//! Validated protocol identifiers and bounded value objects.

use core::fmt;

use crate::generated_contracts::protocol;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueError {
    InvalidLength { expected: usize, actual: usize },
    OutOfRange,
}

impl fmt::Display for ValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid protocol value: {self:?}")
    }
}

impl std::error::Error for ValueError {}

macro_rules! fixed_bytes {
    ($name:ident, $length:expr) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; $length]);

        impl $name {
            pub const LENGTH: usize = $length;

            #[must_use]
            pub const fn new(bytes: [u8; $length]) -> Self {
                Self(bytes)
            }

            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; $length] {
                &self.0
            }

            #[must_use]
            pub const fn into_bytes(self) -> [u8; $length] {
                self.0
            }
        }

        impl TryFrom<&[u8]> for $name {
            type Error = ValueError;

            fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
                let actual = value.len();
                let bytes = value.try_into().map_err(|_| ValueError::InvalidLength {
                    expected: $length,
                    actual,
                })?;
                Ok(Self(bytes))
            }
        }

        impl From<[u8; $length]> for $name {
            fn from(value: [u8; $length]) -> Self {
                Self::new(value)
            }
        }
    };
}

fixed_bytes!(PacketId, 16);
fixed_bytes!(MessageId, 16);
fixed_bytes!(ConversationId, 16);
fixed_bytes!(ContactId, 16);
fixed_bytes!(RoutingSlot, 16);
fixed_bytes!(TransferId, 16);
fixed_bytes!(TokenGrantId, 16);
fixed_bytes!(RandomSourceId, 16);
fixed_bytes!(BootId, 16);
fixed_bytes!(IdentityId, 32);
fixed_bytes!(BpIdentityHash, 32);
fixed_bytes!(WireBundleHash, 32);
fixed_bytes!(PayloadHash, 32);
fixed_bytes!(PeerLinkHash, 32);
fixed_bytes!(CreationSequence, 8);

impl CreationSequence {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value.to_be_bytes())
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        u64::from_be_bytes(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Priority {
    P0 = 0,
    P1 = 1,
    P2 = 2,
    P3 = 3,
}

impl TryFrom<u8> for Priority {
    type Error = ValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::P0),
            1 => Ok(Self::P1),
            2 => Ok(Self::P2),
            3 => Ok(Self::P3),
            _ => Err(ValueError::OutOfRange),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MessageClass {
    Direct = 1,
    CheckIn = 2,
    Sos = 3,
    Receipt = 4,
    Cancel = 5,
}

impl TryFrom<u8> for MessageClass {
    type Error = ValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Direct),
            2 => Ok(Self::CheckIn),
            3 => Ok(Self::Sos),
            4 => Ok(Self::Receipt),
            5 => Ok(Self::Cancel),
            _ => Err(ValueError::OutOfRange),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BundleLifetime(u64);

impl BundleLifetime {
    pub const MIN_MILLIS: u64 = 60_000;
    pub const MAX_MILLIS: u64 = 604_800_000;

    pub fn from_millis(value: u64) -> Result<Self, ValueError> {
        if !(Self::MIN_MILLIS..=Self::MAX_MILLIS).contains(&value) {
            return Err(ValueError::OutOfRange);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CopyTokens(u8);

impl CopyTokens {
    pub fn new(value: u8) -> Result<Self, ValueError> {
        if value == 0 || u64::from(value) > protocol::ROUTING_PRIVATE_SOS_COPY_TOKENS.max(16) {
            return Err(ValueError::OutOfRange);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HopState {
    count: u8,
    limit: u8,
}

impl HopState {
    pub fn new(count: u8, limit: u8) -> Result<Self, ValueError> {
        if limit == 0 || limit > 32 {
            return Err(ValueError::OutOfRange);
        }
        Ok(Self { count, limit })
    }

    #[must_use]
    pub const fn count(self) -> u8 {
        self.count
    }

    #[must_use]
    pub const fn limit(self) -> u8 {
        self.limit
    }

    pub fn increment(self) -> Result<Self, ValueError> {
        let count = self.count.checked_add(1).ok_or(ValueError::OutOfRange)?;
        Ok(Self {
            count,
            limit: self.limit,
        })
    }

    #[must_use]
    pub const fn exhausted_for_relay(self) -> bool {
        self.count.saturating_add(1) >= self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_ids_reject_wrong_length() {
        assert_eq!(
            PacketId::try_from(&[0; 15][..]),
            Err(ValueError::InvalidLength {
                expected: 16,
                actual: 15,
            })
        );
    }

    #[test]
    fn value_boundaries_are_enforced() {
        assert!(BundleLifetime::from_millis(59_999).is_err());
        assert!(BundleLifetime::from_millis(60_000).is_ok());
        assert!(CopyTokens::new(0).is_err());
        assert!(CopyTokens::new(16).is_ok());
        assert!(CopyTokens::new(17).is_err());
        assert!(HopState::new(0, 0).is_err());
        assert!(HopState::new(0, 32).is_ok());
    }
}
