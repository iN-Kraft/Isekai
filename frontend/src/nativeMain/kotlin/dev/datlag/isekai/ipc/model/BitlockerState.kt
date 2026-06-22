package dev.datlag.isekai.ipc.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
enum class BitlockerState {
    @SerialName("Unprotected") Unprotected,
    @SerialName("Protected") Protected,
    @SerialName("Locked") Locked
}