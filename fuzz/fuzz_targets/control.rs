#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_codec::control::ControlPayload;

fuzz_target!(|data: &[u8]| {
    for frame_type in 0x10..=0x1e {
        if frame_type != 0x15 {
            let _ = ControlPayload::decode(frame_type, data);
        }
    }
});
