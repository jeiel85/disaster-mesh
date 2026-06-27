# BLE-CLA v1 Exact Wire Format

Status: normative. All multi-byte integers are unsigned big-endian. Receivers reject non-zero reserved bits, length overflow, allocation beyond negotiated limits, and integer overflow before allocation.

## 1. Layering

```text
GATT operation
  └─ OuterSegment
       └─ logical bytes
            ├─ PlainFrame before Noise, or
            └─ Noise transport ciphertext containing EncryptedFrame
```

A logical frame never mixes Control and Data channels. Android RX/TX characteristic direction does not change the byte format.

## 2. OuterSegment header — 16 bytes

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 0 | 1 | magic | `0xD8` |
| 1 | 1 | version | `0x01` |
| 2 | 1 | flags | bit0 FIRST, bit1 LAST, bits2..7 zero |
| 3 | 1 | channel | 0 CONTROL, 1 DATA |
| 4 | 4 | logical_frame_id | non-zero, unique for link lifetime |
| 8 | 2 | segment_index | 0-based |
| 10 | 2 | segment_count | 1..1024 |
| 12 | 4 | logical_length | 1..65536 |
| 16 | N | segment bytes | exact slice of logical frame |

Rules:

- `segment_index < segment_count`.
- FIRST iff index 0; LAST iff index `segment_count-1`.
- every segment of one frame repeats identical channel, count, and logical length.
- sender chooses a random non-zero initial frame ID and increments modulo 2^32; before reuse/wrap it closes the link.
- duplicate `(frame_id,index)` with identical bytes is idempotent; different bytes is `SEGMENT_CONFLICT` and closes the link.
- out-of-order segments are accepted with a bitmap. Reassembly timeout is 10 seconds from first segment.
- total received bytes must equal logical_length exactly. No padding.
- pre-Noise logical frame maximum is 512 bytes; post-Noise maximum is 65536 bytes and also bounded by negotiated control/data limits.

## 3. PlainFrame — 8-byte header

| Offset | Size | Field |
|---:|---:|---|
| 0 | 1 | magic `0xD7` |
| 1 | 1 | type: VERSION_HELLO=1, NOISE_MESSAGE=2, PLAIN_ERROR=3 |
| 2 | 1 | flags, must be 0 |
| 3 | 1 | header version=1 |
| 4 | 2 | payload length |
| 6 | 2 | sequence |

Sequence starts at 0 per direction and increments by 1. Duplicate exact frame may be ignored; gap, regression with different bytes, or wrap before handshake completion closes the link.

## 4. EncryptedFrame plaintext — 16-byte header

| Offset | Size | Field |
|---:|---:|---|
| 0 | 1 | frame type |
| 1 | 1 | flags; currently 0 |
| 2 | 2 | reserved=0 |
| 4 | 4 | stream ID; 0 control, 1 bundle transfer in v1 |
| 8 | 4 | frame sequence |
| 12 | 4 | payload length |

- frame sequence starts at 0 per stream/direction and increments by 1; no wrap.
- payload length must equal remaining plaintext bytes.
- stream 0 accepts only control payloads; stream 1 accepts BUNDLE_META/CHUNK/COMMIT/ACK/resume/credit related payloads according to session role.
- Noise transport nonce/counter is owned by the Noise library and must also never repeat.

## 5. Frame type codes

`0x10..0x1E` are defined in `docs/04-protocol-ble-cla-v1.md`; their CBOR payloads are defined in `spec/ble-control-v1.cddl`. `BUNDLE_CHUNK` alone uses the binary body defined in docs/04.

Unknown critical frame type, unknown flag, wrong stream, or payload that is not deterministic CBOR produces a generic encrypted ERROR and link close. Implementations do not expose parser detail to peers.

## 6. GATT operation invariant

The Android adapter allows one in-flight GATT operation per link. Every operation carries a `command_id`; the callback must complete exactly that command. Callback without matching in-flight command, double completion, or cross-link completion closes the affected link and emits a redacted diagnostic.
