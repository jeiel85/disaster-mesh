#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_bundle::{Bundle, DmeCiphertext};

fuzz_target!(|data: &[u8]| {
    let _ = Bundle::decode(data);
    let _ = DmeCiphertext::decode(data);
});
