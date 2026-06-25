# ADR-001: Android First

Status: Accepted

## Decision

1.0의 완전한 relay backbone은 Android만 대상으로 한다. iOS는 1.1에서 직접 통신과 best-effort background relay로 추가한다.

## Rationale

- Android는 connected-device foreground service와 사용자 가시적 지속 실행 모델을 제공한다.
- iOS background BLE는 상태·OS 정책에 따라 동작이 제한된다.
- 두 플랫폼을 동시에 시작하면 프로토콜 문제와 OS lifecycle 문제를 분리하기 어렵다.

## Consequence

- protocol/core는 iOS를 고려해 Rust로 공유한다.
- Android-specific API는 adapter 밖으로 새지 않게 한다.
