package org.disastermesh.android.domain

enum class RelayFailureCategory(val display: String) {
    PERMISSION("권한"),
    BLUETOOTH_OFF("Bluetooth 꺼짐"),
    CONNECT_TIMEOUT("연결 시간 초과"),
    HANDSHAKE("보안 세션"),
    PROTOCOL("프로토콜"),
    QUOTA("저장 한도"),
    OTHER("기타"),
}

/** Aggregate-only diagnostics. Peer, packet, contact and message identifiers are absent by type. */
data class RelayDiagnosticsSnapshot(
    val activeLinks: Int,
    val encounters: Long,
    val bytesSent: Long,
    val bytesReceived: Long,
    val bundlesCommitted: Long,
    val partialTransfers: Int,
    val failures: Map<RelayFailureCategory, Long>,
) {
    init {
        require(activeLinks >= 0 && partialTransfers >= 0)
        require(listOf(encounters, bytesSent, bytesReceived, bundlesCommitted).all { it >= 0 })
        require(failures.values.all { it >= 0 })
    }

    fun redactedLines(): List<String> = buildList {
        add("활성 링크: $activeLinks")
        add("접촉 횟수: $encounters")
        add("송신/수신 바이트: $bytesSent / $bytesReceived")
        add("커밋된 번들: $bundlesCommitted")
        add("재개 대기 전송: $partialTransfers")
        failures.toSortedMap().forEach { (category, count) ->
            add("${category.display}: $count")
        }
    }
}
