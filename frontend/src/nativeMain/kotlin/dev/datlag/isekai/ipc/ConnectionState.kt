package dev.datlag.isekai.ipc

import kotlinx.serialization.Serializable

@Serializable
sealed interface ConnectionState {

    @Serializable
    data object Disconnected : ConnectionState

    @Serializable
    data object Connecting : ConnectionState

    @Serializable
    data object Connected : ConnectionState

    @Serializable
    data class Error(val error: IPCError) : ConnectionState
}