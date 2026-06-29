use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mesh_bundle::DmeCiphertext;
use mesh_crypto::{
    ContactCard, CryptoError, DmeAad, EncryptedDme, Identity, MessageBody, open_dme,
};
use mesh_engine::{SecureMessageDraft, create_secure_bundle, open_secure_bundle};
use mesh_types::{
    BundleLifetime, ConversationId, CopyTokens, CreationSequence, HopState, MessageId, PacketId,
    RandomSourceId, RoutingSlot,
};

const TEST_VECTOR_ONLY_MARKER: &str = "DisasterMesh/TEST-VECTOR-ONLY/1";

fn main() -> Result<(), Box<dyn Error>> {
    let command = std::env::args().nth(1).unwrap_or_else(|| "verify".into());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-vectors");
    match command.as_str() {
        "generate" => generate(&root),
        "verify" => verify(&root),
        _ => Err(format!("usage: vector-tool [generate|verify], got {command}").into()),
    }
}

fn identities() -> Result<(Identity, Identity), CryptoError> {
    Ok((
        Identity::from_private_material([1; 32], [2; 32], [3; 32], RoutingSlot::from([4; 16]), 1)?,
        Identity::from_private_material([5; 32], [6; 32], [7; 32], RoutingSlot::from([8; 16]), 1)?,
    ))
}

fn direct_draft() -> SecureMessageDraft {
    SecureMessageDraft {
        packet_id: PacketId::from([0x10; 16]),
        message_id: MessageId::from([0x11; 16]),
        conversation_id: ConversationId::from([0x12; 16]),
        destination: RoutingSlot::from([8; 16]),
        source: RandomSourceId::from([0x13; 16]),
        creation_sequence: CreationSequence::from_u64(0x1415_1617_1819_1a1b),
        lifetime: BundleLifetime::from_millis(259_200_000).expect("valid lifetime"),
        hop_limit: 12,
        copy_tokens: CopyTokens::new(6).expect("valid tokens"),
        sender_sequence: 1,
        created_time_ms: Some(1_782_691_200_000),
        body: MessageBody::DirectText {
            text: "DisasterMesh golden vector".into(),
            reply_to: None,
        },
    }
}

fn receipt_draft() -> SecureMessageDraft {
    SecureMessageDraft {
        packet_id: PacketId::from([0x20; 16]),
        message_id: MessageId::from([0x21; 16]),
        conversation_id: ConversationId::from([0x12; 16]),
        destination: RoutingSlot::from([4; 16]),
        source: RandomSourceId::from([0x22; 16]),
        creation_sequence: CreationSequence::from_u64(0x2324_2526_2728_292a),
        lifetime: BundleLifetime::from_millis(604_800_000).expect("valid lifetime"),
        hop_limit: 16,
        copy_tokens: CopyTokens::new(12).expect("valid tokens"),
        sender_sequence: 1,
        created_time_ms: None,
        body: MessageBody::DeliveryReceipt {
            original_packet_id: PacketId::from([0x10; 16]),
            original_message_id: MessageId::from([0x11; 16]),
            receiver_note: None,
        },
    }
}

fn generate(root: &Path) -> Result<(), Box<dyn Error>> {
    let (sender, recipient) = identities()?;
    let invalid = root.join("invalid");
    fs::create_dir_all(&invalid)?;

    let card = ContactCard::create(&sender, "Alice", 0x1f)?;
    let card_bytes = card.encode()?;
    let qr = card.to_qr()?;
    write_hex(root.join("contact-card-v1.cbor.hex"), &card_bytes)?;
    fs::write(
        root.join("contact-card-v1.json"),
        format!(
            "{{\n  \"display_name\": \"Alice\",\n  \"display_id\": \"{}\",\n  \"safety_number\": \"{}\",\n  \"qr\": \"{}\"\n}}\n",
            card.display_id(),
            mesh_crypto::safety_number(
                &sender.public().signing_public_key,
                &recipient.public().signing_public_key,
            ),
            qr
        ),
    )?;
    let mut unsigned_card = card.clone();
    unsigned_card.signature[0] ^= 1;
    write_hex(
        invalid.join("contact-card-wrong-signature.cbor.hex"),
        &unsigned_card.encode()?,
    )?;

    let direct = create_secure_bundle(&sender, recipient.public(), direct_draft())?;
    let direct_plaintext = direct.plaintext.encode()?;
    write_hex(
        root.join("direct-text-plaintext.cbor.hex"),
        &direct_plaintext,
    )?;
    fs::write(
        root.join("direct-text-plaintext.json"),
        "{\n  \"message_type\": \"DIRECT_TEXT\",\n  \"text\": \"DisasterMesh golden vector\",\n  \"sender_sequence\": 1\n}\n",
    )?;
    let direct_aad = DmeAad {
        packet_id: direct.decoded.bundle.routing.packet_id,
        destination: direct.decoded.bundle.destination,
        message_class: direct.decoded.bundle.routing.message_class,
        priority: direct.decoded.bundle.routing.priority,
        lifetime: direct.decoded.bundle.lifetime,
        hop_limit: direct.decoded.bundle.hops.limit(),
        source: direct.decoded.bundle.source,
        creation_sequence: direct.decoded.bundle.creation_sequence,
    };
    write_hex(root.join("direct-text-aad.hex"), &direct_aad.encode()?)?;
    let envelope = DmeCiphertext::decode(&direct.decoded.bundle.payload)?;
    write_hex(
        root.join("direct-text-hpke.enc.hex"),
        &envelope.encapsulated_key,
    )?;
    write_hex(
        root.join("direct-text-ciphertext.cbor.hex"),
        &direct.decoded.bundle.payload,
    )?;
    write_hex(root.join("direct-text-bpv7.hex"), &direct.wire_bytes)?;
    let mut wrong_signature = direct.plaintext.clone();
    wrong_signature.signature[0] ^= 1;
    write_hex(
        invalid.join("direct-text-wrong-signature-plaintext.cbor.hex"),
        &wrong_signature.encode()?,
    )?;

    let receipt = create_secure_bundle(&recipient, sender.public(), receipt_draft())?;
    write_hex(root.join("receipt-bpv7.hex"), &receipt.wire_bytes)?;

    let mut wrong_aad = direct.decoded.bundle.clone();
    wrong_aad.hops = HopState::new(0, 13).expect("valid tamper");
    write_hex(
        invalid.join("direct-text-wrong-aad-bpv7.hex"),
        &wrong_aad.encode()?,
    )?;
    let mut truncated = direct.decoded.bundle.payload.clone();
    truncated.pop();
    write_hex(invalid.join("direct-text-truncated.cbor.hex"), &truncated)?;
    let mut bad_qr = qr.into_bytes();
    let last = bad_qr.len() - 1;
    bad_qr[last] = if bad_qr[last] == b'0' { b'1' } else { b'0' };
    fs::write(invalid.join("contact-card-bad-crc.txt"), bad_qr)?;

    fs::write(
        root.join("goal-2-manifest.json"),
        format!(
            "{{\n  \"name\": \"goal-2-identity-e2ee\",\n  \"protocol\": \"DME v1 / DM-BP7-1 / RFC 9180\",\n  \"description\": \"Real Ed25519, X25519 HPKE and invalid mutation vectors\",\n  \"inputs\": {{\"identity_fixture\": \"fixed non-production key material\", \"hpke_ephemeral\": \"OS CSPRNG output captured in committed ciphertext\", \"marker\": \"{TEST_VECTOR_ONLY_MARKER}\"}},\n  \"outputs\": {{\"direct_bundle\": \"direct-text-bpv7.hex\", \"receipt_bundle\": \"receipt-bpv7.hex\", \"contact_card\": \"contact-card-v1.cbor.hex\"}},\n  \"expected_error\": null,\n  \"generator_commit\": \"goal-2-local\"\n}}\n"
        ),
    )?;
    verify(root)
}

fn verify(root: &Path) -> Result<(), Box<dyn Error>> {
    let (sender, recipient) = identities()?;
    let card = ContactCard::decode(&read_hex(root.join("contact-card-v1.cbor.hex"))?)?;
    if card.signing_public_key != sender.public().signing_public_key {
        return Err("contact vector signer mismatch".into());
    }

    let direct_wire = read_hex(root.join("direct-text-bpv7.hex"))?;
    let (direct_bundle, plaintext) = open_secure_bundle(&recipient, &direct_wire)?;
    if plaintext.encode()? != read_hex(root.join("direct-text-plaintext.cbor.hex"))? {
        return Err("direct plaintext vector mismatch".into());
    }
    if direct_bundle.bundle.payload != read_hex(root.join("direct-text-ciphertext.cbor.hex"))? {
        return Err("direct ciphertext vector mismatch".into());
    }
    let aad = DmeAad {
        packet_id: direct_bundle.bundle.routing.packet_id,
        destination: direct_bundle.bundle.destination,
        message_class: direct_bundle.bundle.routing.message_class,
        priority: direct_bundle.bundle.routing.priority,
        lifetime: direct_bundle.bundle.lifetime,
        hop_limit: direct_bundle.bundle.hops.limit(),
        source: direct_bundle.bundle.source,
        creation_sequence: direct_bundle.bundle.creation_sequence,
    };
    if aad.encode()? != read_hex(root.join("direct-text-aad.hex"))? {
        return Err("AAD vector mismatch".into());
    }
    let envelope = EncryptedDme::decode(&direct_bundle.bundle.payload)?;
    if envelope.encapsulated_key != read_hex(root.join("direct-text-hpke.enc.hex"))?.as_slice() {
        return Err("HPKE encapsulated key mismatch".into());
    }
    if open_secure_bundle(&sender, &direct_wire).is_ok() {
        return Err("wrong recipient key accepted direct vector".into());
    }

    let wrong_signature = mesh_crypto::DmePlaintext::decode(&read_hex(
        root.join("invalid/direct-text-wrong-signature-plaintext.cbor.hex"),
    )?)?;
    if wrong_signature.verify_signature(&envelope.aad_hash).is_ok() {
        return Err("wrong DME signature vector was accepted".into());
    }

    let receipt_wire = read_hex(root.join("receipt-bpv7.hex"))?;
    let (_, receipt) = open_secure_bundle(&sender, &receipt_wire)?;
    if !matches!(receipt.body, MessageBody::DeliveryReceipt { .. }) {
        return Err("receipt vector did not decode as receipt".into());
    }

    let wrong_aad = read_hex(root.join("invalid/direct-text-wrong-aad-bpv7.hex"))?;
    if open_secure_bundle(&recipient, &wrong_aad).is_ok() {
        return Err("wrong AAD vector was accepted".into());
    }
    let truncated = read_hex(root.join("invalid/direct-text-truncated.cbor.hex"))?;
    if EncryptedDme::decode(&truncated).is_ok() {
        return Err("truncated ciphertext vector was accepted".into());
    }
    let bad_qr = fs::read_to_string(root.join("invalid/contact-card-bad-crc.txt"))?;
    if ContactCard::from_qr(&bad_qr).is_ok() {
        return Err("bad contact CRC vector was accepted".into());
    }
    if ContactCard::decode(&read_hex(
        root.join("invalid/contact-card-wrong-signature.cbor.hex"),
    )?)
    .is_ok()
    {
        return Err("bad contact signature vector was accepted".into());
    }

    let dme = DmeCiphertext::decode(&direct_bundle.bundle.payload)?;
    let encrypted = EncryptedDme {
        encapsulated_key: dme.encapsulated_key,
        aad_hash: dme.aad_hash,
        ciphertext: dme.ciphertext,
    };
    open_dme(&recipient, aad, &encrypted)?;
    println!("Goal 2 vectors verified in a separate process");
    Ok(())
}

fn write_hex(path: PathBuf, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    fs::write(path, format!("{}\n", hex_encode(bytes)))?;
    Ok(())
}

fn read_hex(path: PathBuf) -> Result<Vec<u8>, Box<dyn Error>> {
    let input = fs::read_to_string(path)?;
    hex_decode(input.trim())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(input: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !input.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    let mut output = Vec::with_capacity(input.len() / 2);
    for pair in input.as_bytes().chunks_exact(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_value(value: u8) -> Result<u8, Box<dyn Error>> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("non-lowercase-hex input".into()),
    }
}
