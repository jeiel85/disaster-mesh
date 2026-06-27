-- Run after creating sqlite_v1.sql. Every query must return zero rows.
SELECT 'foreign_key_violation' AS violation, * FROM pragma_foreign_key_check;

SELECT 'invalid_outbound_message_state', hex(message_id)
FROM local_messages
WHERE direction = 0 AND delivery_state NOT IN (0,1,2,3,4,5,6,9,10);

SELECT 'invalid_inbound_message_state', hex(message_id)
FROM local_messages
WHERE direction = 1 AND delivery_state NOT IN (7,8,10);

SELECT 'invalid_replay_bitmap', hex(contact_id)
FROM contact_replay_state
WHERE length(seen_bitmap) <> 512 OR window_base_sequence > max_sender_sequence;

SELECT 'invalid_partial_bitmap', hex(transfer_id)
FROM transfers
WHERE received_bitmap IS NOT NULL AND length(received_bitmap) <> ((chunk_count + 7) / 8);

SELECT 'terminal_grant_missing_commit_evidence', hex(grant_id)
FROM token_grants
WHERE state = 2 AND (committed_payload_sha256 IS NULL OR committed_wire_sha256 IS NULL OR accepted_tokens IS NULL OR committed_at_ms IS NULL);
