//! Persistable 4096-bit sender-sequence replay window.

use mesh_types::generated_contracts::protocol;

pub const WINDOW_BITS: usize = protocol::REPLAY_WINDOW_BITS as usize;
pub const WINDOW_BYTES: usize = protocol::REPLAY_WINDOW_BYTES as usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayDecision {
    AcceptedNewMaximum,
    AcceptedDelayed,
    Replay,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayWindow {
    max_sender_sequence: u64,
    seen_bitmap: [u8; WINDOW_BYTES],
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self {
            max_sender_sequence: 0,
            seen_bitmap: [0; WINDOW_BYTES],
        }
    }
}

impl ReplayWindow {
    #[must_use]
    pub fn from_persisted(max_sender_sequence: u64, seen_bitmap: [u8; WINDOW_BYTES]) -> Self {
        Self {
            max_sender_sequence,
            seen_bitmap,
        }
    }

    #[must_use]
    pub const fn max_sender_sequence(&self) -> u64 {
        self.max_sender_sequence
    }

    #[must_use]
    pub const fn bitmap(&self) -> &[u8; WINDOW_BYTES] {
        &self.seen_bitmap
    }

    pub fn observe(&mut self, sequence: u64) -> ReplayDecision {
        if !self.initialized() {
            self.max_sender_sequence = sequence;
            self.set(0);
            return ReplayDecision::AcceptedNewMaximum;
        }

        if sequence > self.max_sender_sequence {
            let shift = sequence - self.max_sender_sequence;
            if shift >= WINDOW_BITS as u64 {
                self.seen_bitmap.fill(0);
            } else {
                self.shift_older(shift as usize);
            }
            self.max_sender_sequence = sequence;
            self.set(0);
            return ReplayDecision::AcceptedNewMaximum;
        }

        let offset = self.max_sender_sequence - sequence;
        if offset >= WINDOW_BITS as u64 {
            return ReplayDecision::Stale;
        }
        if self.get(offset as usize) {
            ReplayDecision::Replay
        } else {
            self.set(offset as usize);
            ReplayDecision::AcceptedDelayed
        }
    }

    fn initialized(&self) -> bool {
        self.seen_bitmap.iter().any(|byte| *byte != 0)
    }

    fn get(&self, offset: usize) -> bool {
        self.seen_bitmap[offset / 8] & (1 << (offset % 8)) != 0
    }

    fn set(&mut self, offset: usize) {
        self.seen_bitmap[offset / 8] |= 1 << (offset % 8);
    }

    fn shift_older(&mut self, shift: usize) {
        let previous = self.seen_bitmap;
        self.seen_bitmap.fill(0);
        for old_offset in 0..(WINDOW_BITS - shift) {
            if previous[old_offset / 8] & (1 << (old_offset % 8)) != 0 {
                self.set(old_offset + shift);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn accepts_delayed_once_inside_window() {
        let mut window = ReplayWindow::default();
        assert_eq!(window.observe(10), ReplayDecision::AcceptedNewMaximum);
        assert_eq!(window.observe(8), ReplayDecision::AcceptedDelayed);
        assert_eq!(window.observe(8), ReplayDecision::Replay);
    }

    #[test]
    fn rejects_outside_window() {
        let mut window = ReplayWindow::default();
        assert_eq!(window.observe(1), ReplayDecision::AcceptedNewMaximum);
        assert_eq!(window.observe(4096), ReplayDecision::AcceptedNewMaximum);
        assert_eq!(window.observe(0), ReplayDecision::Stale);
    }

    proptest! {
        #[test]
        fn an_observed_sequence_is_never_accepted_twice(sequence in 0u64..100_000) {
            let mut window = ReplayWindow::default();
            prop_assert!(matches!(
                window.observe(sequence),
                ReplayDecision::AcceptedNewMaximum | ReplayDecision::AcceptedDelayed
            ));
            prop_assert_eq!(window.observe(sequence), ReplayDecision::Replay);
        }

        #[test]
        fn persisted_round_trip_preserves_decisions(sequences in prop::collection::vec(0u64..20_000, 1..200)) {
            let mut original = ReplayWindow::default();
            for sequence in &sequences {
                original.observe(*sequence);
            }
            let mut restored = ReplayWindow::from_persisted(
                original.max_sender_sequence(),
                *original.bitmap(),
            );
            for sequence in sequences {
                prop_assert_eq!(original.observe(sequence), restored.observe(sequence));
            }
        }
    }
}
