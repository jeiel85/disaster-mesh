//! Pure terminal-receipt and reordered-cancel state rules.

use std::collections::BTreeMap;

pub type Id16 = [u8; 16];
pub type IdentityHash = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MessageType {
    DirectText = 1,
    CheckIn = 2,
    PrivateSos = 3,
    LocationUpdate = 4,
    DeliveryReceipt = 5,
    Cancel = 6,
}

#[must_use]
pub const fn should_generate_receipt(message_type: MessageType) -> bool {
    !matches!(message_type, MessageType::DeliveryReceipt)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OriginalKey {
    pub packet_id: Id16,
    pub message_id: Id16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Original {
    pub key: OriginalKey,
    pub sender: IdentityHash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CancelControl {
    pub cancel_packet_id: Id16,
    pub cancel_message_id: Id16,
    pub target: OriginalKey,
    pub verified_sender: IdentityHash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelOutcome {
    Pending,
    Applied,
    Duplicate,
    ConflictQuarantined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalOutcome {
    Visible,
    CanceledBeforeExposure,
    VisibleAndConflictQuarantined,
    Duplicate,
}

#[derive(Default)]
pub struct ControlState {
    originals: BTreeMap<OriginalKey, Original>,
    pending: BTreeMap<OriginalKey, CancelControl>,
    applied: BTreeMap<OriginalKey, CancelControl>,
}

impl ControlState {
    pub fn receive_cancel(&mut self, cancel: CancelControl) -> CancelOutcome {
        if let Some(applied) = self.applied.get(&cancel.target) {
            return if applied == &cancel {
                CancelOutcome::Duplicate
            } else {
                CancelOutcome::ConflictQuarantined
            };
        }
        if let Some(original) = self.originals.get(&cancel.target) {
            if original.sender != cancel.verified_sender {
                return CancelOutcome::ConflictQuarantined;
            }
            self.applied.insert(cancel.target, cancel);
            return CancelOutcome::Applied;
        }
        if let Some(existing) = self.pending.get(&cancel.target) {
            return if existing == &cancel {
                CancelOutcome::Duplicate
            } else {
                CancelOutcome::ConflictQuarantined
            };
        }
        self.pending.insert(cancel.target, cancel);
        CancelOutcome::Pending
    }

    pub fn receive_original(&mut self, original: Original) -> OriginalOutcome {
        if self.originals.contains_key(&original.key) {
            return OriginalOutcome::Duplicate;
        }
        let pending = self.pending.remove(&original.key);
        self.originals.insert(original.key, original);
        match pending {
            Some(cancel) if cancel.verified_sender == original.sender => {
                self.applied.insert(original.key, cancel);
                OriginalOutcome::CanceledBeforeExposure
            }
            Some(_) => OriginalOutcome::VisibleAndConflictQuarantined,
            None => OriginalOutcome::Visible,
        }
    }

    #[must_use]
    pub fn is_canceled(&self, key: OriginalKey) -> bool {
        self.applied.contains_key(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: u8) -> OriginalKey {
        OriginalKey {
            packet_id: [value; 16],
            message_id: [value.wrapping_add(1); 16],
        }
    }

    fn cancel(target: OriginalKey, sender: u8) -> CancelControl {
        CancelControl {
            cancel_packet_id: [9; 16],
            cancel_message_id: [10; 16],
            target,
            verified_sender: [sender; 32],
        }
    }

    #[test]
    fn receipt_is_terminal_and_cancel_receipt_is_terminal_next() {
        assert!(!should_generate_receipt(MessageType::DeliveryReceipt));
        assert!(should_generate_receipt(MessageType::Cancel));
        assert!(!should_generate_receipt(MessageType::DeliveryReceipt));
    }

    #[test]
    fn cancel_before_original_applies_before_exposure() {
        let target = key(1);
        let mut state = ControlState::default();
        assert_eq!(
            state.receive_cancel(cancel(target, 3)),
            CancelOutcome::Pending
        );
        assert_eq!(
            state.receive_original(Original {
                key: target,
                sender: [3; 32],
            }),
            OriginalOutcome::CanceledBeforeExposure
        );
        assert!(state.is_canceled(target));
    }

    #[test]
    fn original_before_cancel_and_duplicate_are_idempotent() {
        let target = key(2);
        let mut state = ControlState::default();
        assert_eq!(
            state.receive_original(Original {
                key: target,
                sender: [4; 32],
            }),
            OriginalOutcome::Visible
        );
        let control = cancel(target, 4);
        assert_eq!(state.receive_cancel(control), CancelOutcome::Applied);
        assert_eq!(state.receive_cancel(control), CancelOutcome::Duplicate);
    }

    #[test]
    fn forged_cancel_never_changes_original() {
        let target = key(3);
        let mut state = ControlState::default();
        assert_eq!(
            state.receive_cancel(cancel(target, 7)),
            CancelOutcome::Pending
        );
        assert_eq!(
            state.receive_original(Original {
                key: target,
                sender: [8; 32],
            }),
            OriginalOutcome::VisibleAndConflictQuarantined
        );
        assert!(!state.is_canceled(target));
    }
}
