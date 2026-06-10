package dev.datlag.isekai.ipc

import kotlinx.serialization.Serializable

@Serializable
sealed interface IPCError {

    @Serializable
    data class ConnectionFailed(val reason: String, val win32Error: UInt? = null) : IPCError

    @Serializable
    data class Disconnected(val reason: String) : IPCError

    @Serializable
    data class SerializationError(val reason: String) : IPCError

    @Serializable
    data class RequestCancelled(val id: String) : IPCError

    @Serializable
    data class BackendError(val message: String) : IPCError
}