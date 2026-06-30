#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_codec::ble::{OuterReassembler, OuterSegment};

fuzz_target!(|data: &[u8]| {
    let mut reassembler = OuterReassembler::new(8, 8, 10_000);
    let mut cursor = 0usize;
    let mut now = 0u64;
    while cursor + 2 <= data.len() {
        let length = usize::from(u16::from_be_bytes([data[cursor], data[cursor + 1]]));
        cursor += 2;
        let Some(end) = cursor.checked_add(length).filter(|end| *end <= data.len()) else {
            break;
        };
        if let Ok(segment) = OuterSegment::decode(&data[cursor..end]) {
            let _ = reassembler.accept(segment, now);
        }
        cursor = end;
        now = now.saturating_add(1);
        let _ = reassembler.expire(now);
    }
});
