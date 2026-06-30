package org.disastermesh.android.transport

sealed interface MtuRequestDecision {
    data class Request(val mtu: Int = 517) : MtuRequestDecision
    data object AlreadyRequested : MtuRequestDecision
}

class MtuNegotiation {
    private val requestedLinks = mutableSetOf<Long>()

    fun begin(linkId: Long): MtuRequestDecision = if (requestedLinks.add(linkId)) {
        MtuRequestDecision.Request()
    } else {
        MtuRequestDecision.AlreadyRequested
    }

    fun applicationPayload(actualMtu: Int, remoteMaximum: Int): Int? {
        val local = (actualMtu - ATT_OVERHEAD).coerceAtMost(remoteMaximum)
        return local.takeIf { it >= MINIMUM_APPLICATION_PAYLOAD }
    }

    fun close(linkId: Long) {
        requestedLinks.remove(linkId)
    }

    companion object {
        const val DEFAULT_ATT_MTU = 23
        const val ATT_OVERHEAD = 3
        const val MINIMUM_APPLICATION_PAYLOAD = 20
    }
}
