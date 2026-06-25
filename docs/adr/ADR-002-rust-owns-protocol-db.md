# ADR-002: Rust Core Owns the Protocol Database

Status: Accepted

## Decision

Rust core가 SQLite protocol database를 직접 소유한다. Android Room은 protocol state에 사용하지 않는다.

## Rationale

- copy-token commit과 bundle commit의 원자성
- Android/iOS/Linux 간 동일 migration/logic
- FFI를 통한 세부 repository callback 제거
- simulator와 production state model의 일치

## Consequence

- UI query DTO를 FFI로 제공한다.
- DB master key는 Android Keystore가 unwrap한 뒤 Rust에 전달한다.
- Rust SQLite Android/iOS packaging을 CI에서 검증해야 한다.
