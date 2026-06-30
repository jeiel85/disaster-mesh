#![no_main]

use libfuzzer_sys::fuzz_target;
use mesh_crypto::{ColumnContext, ContactCard, DbMasterKey, DmePlaintext, decrypt_local_value};

fuzz_target!(|data: &[u8]| {
    let _ = ContactCard::decode(data);
    let _ = DmePlaintext::decode(data);
    let key = DbMasterKey::from_bytes([0; 32]);
    let _ = decrypt_local_value(
        &key,
        ColumnContext {
            schema_version: 1,
            table: "fuzz",
            column: "value",
            primary_key: b"key",
            key_version: 1,
            identity_hash: &[0; 32],
        },
        data,
    );
});
