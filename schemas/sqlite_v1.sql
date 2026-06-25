PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA auto_vacuum = INCREMENTAL;
PRAGMA user_version = 1;

CREATE TABLE schema_meta (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE runtime_checkpoint (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    boot_id BLOB NOT NULL CHECK (length(boot_id) = 16),
    elapsed_ms INTEGER NOT NULL CHECK (elapsed_ms >= 0),
    wall_ms INTEGER,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE identity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    identity_hash BLOB NOT NULL CHECK (length(identity_hash) = 32),
    signing_public_key BLOB NOT NULL CHECK (length(signing_public_key) = 32),
    hpke_public_key BLOB NOT NULL CHECK (length(hpke_public_key) = 32),
    noise_public_key BLOB NOT NULL CHECK (length(noise_public_key) = 32),
    encrypted_private_keys BLOB NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE inbound_routing_slots (
    routing_slot BLOB PRIMARY KEY NOT NULL CHECK (length(routing_slot) = 16),
    state INTEGER NOT NULL CHECK (state BETWEEN 0 AND 2),
    key_version INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    retire_after_ms INTEGER
);
CREATE UNIQUE INDEX idx_one_active_inbound_slot
    ON inbound_routing_slots(state) WHERE state = 0;
CREATE INDEX idx_inbound_slots_state_expiry
    ON inbound_routing_slots(state, retire_after_ms);

CREATE TABLE contacts (
    contact_id BLOB PRIMARY KEY NOT NULL CHECK (length(contact_id) = 16),
    identity_hash BLOB NOT NULL UNIQUE CHECK (length(identity_hash) = 32),
    signing_public_key BLOB NOT NULL CHECK (length(signing_public_key) = 32),
    hpke_public_key BLOB NOT NULL CHECK (length(hpke_public_key) = 32),
    destination_routing_slot BLOB NOT NULL CHECK (length(destination_routing_slot) = 16),
    encrypted_display_name BLOB NOT NULL,
    trust_state INTEGER NOT NULL CHECK (trust_state BETWEEN 0 AND 3),
    safety_verified INTEGER NOT NULL CHECK (safety_verified IN (0,1)),
    outbound_sender_sequence INTEGER NOT NULL DEFAULT 0 CHECK (outbound_sender_sequence >= 0),
    key_version INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    revoked_at_ms INTEGER
);
CREATE INDEX idx_contacts_routing_slot ON contacts(destination_routing_slot);

CREATE TABLE contact_replay_state (
    contact_id BLOB PRIMARY KEY NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
    max_sender_sequence INTEGER NOT NULL DEFAULT 0 CHECK (max_sender_sequence >= 0),
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE contact_replay_window (
    contact_id BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
    message_id BLOB NOT NULL CHECK (length(message_id) = 16),
    packet_id BLOB NOT NULL CHECK (length(packet_id) = 16),
    sender_sequence INTEGER NOT NULL CHECK (sender_sequence >= 0),
    received_at_ms INTEGER NOT NULL,
    PRIMARY KEY (contact_id, message_id),
    UNIQUE (contact_id, packet_id)
);
CREATE INDEX idx_replay_window_sequence
    ON contact_replay_window(contact_id, sender_sequence DESC);

CREATE TABLE conversations (
    conversation_id BLOB PRIMARY KEY NOT NULL CHECK (length(conversation_id) = 16),
    contact_id BLOB NOT NULL REFERENCES contacts(contact_id),
    created_at_ms INTEGER NOT NULL,
    last_message_at_ms INTEGER NOT NULL,
    UNIQUE(contact_id)
);

CREATE TABLE send_groups (
    send_group_id BLOB PRIMARY KEY NOT NULL CHECK (length(send_group_id) = 16),
    message_type INTEGER NOT NULL CHECK (message_type BETWEEN 1 AND 6),
    recipient_count INTEGER NOT NULL CHECK (recipient_count BETWEEN 1 AND 16),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE local_messages (
    message_id BLOB PRIMARY KEY NOT NULL CHECK (length(message_id) = 16),
    conversation_id BLOB NOT NULL REFERENCES conversations(conversation_id),
    packet_id BLOB NOT NULL UNIQUE CHECK (length(packet_id) = 16),
    direction INTEGER NOT NULL CHECK (direction IN (0,1)),
    message_type INTEGER NOT NULL CHECK (message_type BETWEEN 1 AND 6),
    send_group_id BLOB REFERENCES send_groups(send_group_id),
    encrypted_body BLOB NOT NULL,
    delivery_state INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    received_at_ms INTEGER,
    expires_at_estimate_ms INTEGER NOT NULL,
    reply_to_message_id BLOB,
    cancel_reason INTEGER
);
CREATE INDEX idx_messages_conversation_time ON local_messages(conversation_id, created_at_ms DESC);
CREATE INDEX idx_messages_state ON local_messages(delivery_state);

CREATE TABLE bundles (
    packet_id BLOB PRIMARY KEY NOT NULL CHECK (length(packet_id) = 16),
    bp_identity_hash BLOB NOT NULL UNIQUE CHECK (length(bp_identity_hash) = 32),
    destination_slot BLOB NOT NULL CHECK (length(destination_slot) = 16),
    random_source_id BLOB NOT NULL CHECK (length(random_source_id) = 16),
    creation_sequence BLOB NOT NULL CHECK (length(creation_sequence) = 8),
    message_class_hint INTEGER NOT NULL CHECK (message_class_hint BETWEEN 1 AND 5),
    priority INTEGER NOT NULL CHECK (priority BETWEEN 0 AND 3),
    lifetime_ms INTEGER NOT NULL CHECK (lifetime_ms BETWEEN 60000 AND 604800000),
    stored_age_ms INTEGER NOT NULL CHECK (stored_age_ms >= 0),
    age_anchor_elapsed_ms INTEGER NOT NULL CHECK (age_anchor_elapsed_ms >= 0),
    received_boot_id BLOB NOT NULL CHECK (length(received_boot_id) = 16),
    hop_count INTEGER NOT NULL CHECK (hop_count >= 0),
    hop_limit INTEGER NOT NULL CHECK (hop_limit BETWEEN 1 AND 32),
    copy_tokens INTEGER NOT NULL CHECK (copy_tokens BETWEEN 1 AND 16),
    payload_size INTEGER NOT NULL CHECK (payload_size BETWEEN 1 AND 8192),
    payload_sha256 BLOB NOT NULL CHECK (length(payload_sha256) = 32),
    wire_sha256 BLOB NOT NULL CHECK (length(wire_sha256) = 32),
    state INTEGER NOT NULL,
    origin INTEGER NOT NULL CHECK (origin BETWEEN 0 AND 2),
    created_local_ms INTEGER NOT NULL,
    last_offered_ms INTEGER,
    ingress_peer_hash BLOB,
    failure_count INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_bundles_queue ON bundles(state, priority, created_local_ms);
CREATE INDEX idx_bundles_destination ON bundles(destination_slot, state);
CREATE INDEX idx_bundles_expiry ON bundles(state, lifetime_ms, stored_age_ms);

CREATE TABLE bundle_payloads (
    packet_id BLOB PRIMARY KEY NOT NULL REFERENCES bundles(packet_id) ON DELETE CASCADE,
    bp_bundle_bytes BLOB NOT NULL,
    CHECK(length(bp_bundle_bytes) <= 12288)
);

CREATE TABLE transfers (
    transfer_id BLOB PRIMARY KEY NOT NULL CHECK (length(transfer_id) = 16),
    packet_id BLOB NOT NULL CHECK (length(packet_id) = 16),
    token_grant_id BLOB CHECK (token_grant_id IS NULL OR length(token_grant_id) = 16),
    link_id INTEGER NOT NULL,
    direction INTEGER NOT NULL CHECK (direction IN (0,1)),
    state INTEGER NOT NULL,
    total_size INTEGER NOT NULL CHECK (total_size BETWEEN 1 AND 12288),
    chunk_size INTEGER NOT NULL CHECK (chunk_size > 0),
    chunk_count INTEGER NOT NULL CHECK (chunk_count BETWEEN 1 AND 1024),
    received_bitmap BLOB,
    proposed_receiver_tokens INTEGER,
    sender_tokens_after_reservation INTEGER,
    started_elapsed_ms INTEGER NOT NULL,
    updated_elapsed_ms INTEGER NOT NULL,
    temp_path TEXT
);
CREATE INDEX idx_transfers_updated ON transfers(updated_elapsed_ms);

CREATE TABLE token_grants (
    grant_id BLOB PRIMARY KEY NOT NULL CHECK (length(grant_id) = 16),
    packet_id BLOB NOT NULL CHECK (length(packet_id) = 16),
    peer_link_hash BLOB NOT NULL,
    direction INTEGER NOT NULL CHECK (direction IN (0,1)),
    state INTEGER NOT NULL CHECK (state BETWEEN 0 AND 3),
    token_count INTEGER NOT NULL CHECK (token_count BETWEEN 1 AND 15),
    transfer_id BLOB CHECK (transfer_id IS NULL OR length(transfer_id) = 16),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    retain_until_ms INTEGER NOT NULL,
    UNIQUE(packet_id, grant_id, direction)
);
CREATE INDEX idx_token_grants_packet_state
    ON token_grants(packet_id, direction, state);
CREATE INDEX idx_token_grants_peer_state
    ON token_grants(peer_link_hash, state);

CREATE TABLE receipts (
    original_packet_id BLOB PRIMARY KEY NOT NULL CHECK (length(original_packet_id) = 16),
    receipt_packet_id BLOB NOT NULL UNIQUE CHECK (length(receipt_packet_id) = 16),
    verified INTEGER NOT NULL CHECK (verified IN (0,1)),
    received_at_ms INTEGER NOT NULL
);

CREATE TABLE tombstones (
    packet_id BLOB PRIMARY KEY NOT NULL CHECK (length(packet_id) = 16),
    payload_sha256 BLOB CHECK (payload_sha256 IS NULL OR length(payload_sha256) = 32),
    reason INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_tombstones_expiry ON tombstones(expires_at_ms);

CREATE TABLE peer_encounters (
    encounter_id INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_link_hash BLOB NOT NULL,
    beacon_id BLOB,
    first_seen_elapsed_ms INTEGER NOT NULL,
    last_seen_elapsed_ms INTEGER NOT NULL,
    connect_result INTEGER NOT NULL,
    bytes_sent INTEGER NOT NULL DEFAULT 0,
    bytes_received INTEGER NOT NULL DEFAULT 0,
    bundles_committed INTEGER NOT NULL DEFAULT 0,
    failure_category INTEGER
);
CREATE INDEX idx_peer_encounters_peer_time ON peer_encounters(peer_link_hash, last_seen_elapsed_ms DESC);

CREATE TABLE peer_limits (
    peer_link_hash BLOB PRIMARY KEY NOT NULL,
    window_start_ms INTEGER NOT NULL,
    ingested_bytes INTEGER NOT NULL DEFAULT 0,
    unverified_direct_bytes INTEGER NOT NULL DEFAULT 0,
    invalid_packets INTEGER NOT NULL DEFAULT 0,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    cooldown_until_elapsed_ms INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE diagnostic_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at_ms INTEGER NOT NULL,
    category INTEGER NOT NULL,
    severity INTEGER NOT NULL,
    redacted_ref TEXT,
    numeric_value INTEGER,
    detail_code INTEGER
);
CREATE INDEX idx_diagnostic_time ON diagnostic_events(created_at_ms DESC);

INSERT INTO schema_meta(key, value) VALUES ('schema_version', '1');
INSERT INTO schema_meta(key, value) VALUES ('protocol_major', '1');
