use cddl_cat::{flatten::flatten_from_str, validate_cbor_bytes};

const DME: &str = include_str!("../../../spec/dme-v1.cddl");
const DME_AAD: &str = include_str!("../../../spec/dme-aad-v1.cddl");
const CONTACT: &str = include_str!("../../../spec/contact-card-v1.cddl");
const ROUTING: &str = include_str!("../../../spec/disaster-routing-block-v1.cddl");
const BLE_CONTROL: &str = include_str!("../../../spec/ble-control-v1.cddl");

fn array(length: u8, items: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
    let mut output = vec![0x80 | length];
    for item in items {
        output.extend(item);
    }
    output
}

fn uint(value: u64) -> Vec<u8> {
    match value {
        0..=23 => vec![value as u8],
        24..=0xff => vec![0x18, value as u8],
        0x100..=0xffff => {
            let mut result = vec![0x19];
            result.extend_from_slice(&(value as u16).to_be_bytes());
            result
        }
        _ => {
            let mut result = vec![0x1a];
            result.extend_from_slice(&(value as u32).to_be_bytes());
            result
        }
    }
}

fn bytes(length: usize, value: u8) -> Vec<u8> {
    let mut output = if length <= 23 {
        vec![0x40 | length as u8]
    } else {
        vec![0x58, length as u8]
    };
    output.extend(std::iter::repeat_n(value, length));
    output
}

#[test]
fn every_normative_cddl_document_parses() {
    for (name, schema) in [
        ("dme-v1", DME),
        ("dme-aad-v1", DME_AAD),
        ("contact-card-v1", CONTACT),
        ("disaster-routing-block-v1", ROUTING),
        ("ble-control-v1", BLE_CONTROL),
    ] {
        flatten_from_str(schema).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn representative_values_validate_against_each_contract() {
    let dme_ciphertext = array(
        5,
        [uint(1), uint(1), bytes(32, 1), bytes(32, 2), bytes(1, 3)],
    );
    validate_cbor_bytes("dme-ciphertext", DME, &dme_ciphertext).unwrap();

    let aad = array(
        9,
        [
            uint(1),
            bytes(16, 1),
            bytes(16, 2),
            uint(1),
            uint(0),
            uint(60_000),
            uint(1),
            bytes(16, 3),
            bytes(8, 4),
        ],
    );
    validate_cbor_bytes("dme-aad-v1", DME_AAD, &aad).unwrap();

    let contact = array(
        8,
        [
            uint(1),
            bytes(32, 1),
            bytes(32, 2),
            bytes(16, 3),
            vec![0x60],
            uint(1),
            uint(0),
            bytes(64, 4),
        ],
    );
    validate_cbor_bytes("contact-card", CONTACT, &contact).unwrap();

    let routing = array(
        7,
        [
            uint(1),
            bytes(16, 1),
            uint(1),
            uint(0),
            uint(1),
            uint(1),
            bytes(32, 2),
        ],
    );
    validate_cbor_bytes("routing-block", ROUTING, &routing).unwrap();

    let hello = array(
        7,
        [
            uint(1),
            uint(0),
            bytes(8, 1),
            uint(64),
            uint(1),
            uint(0),
            bytes(16, 2),
        ],
    );
    validate_cbor_bytes("version-hello", BLE_CONTROL, &hello).unwrap();
}
