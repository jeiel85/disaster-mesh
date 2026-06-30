#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_codec::{DecodeLimits, decode_deterministic};

fuzz_target!(|data: &[u8]| {
    let _ = decode_deterministic(data, DecodeLimits::default());
});
