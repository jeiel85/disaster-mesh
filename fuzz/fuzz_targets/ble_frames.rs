#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_codec::ble::{BundleChunk, EncryptedFrame, OuterSegment, PlainFrame};

fuzz_target!(|data: &[u8]| {
    let _ = OuterSegment::decode(data);
    let _ = PlainFrame::decode(data);
    let _ = EncryptedFrame::decode(data);
    let _ = BundleChunk::decode(data);
});
