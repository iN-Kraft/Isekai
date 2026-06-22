package dev.datlag.isekai.ipc

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed interface OutgoingMessage {

    @Serializable
    @SerialName("Response")
    data class Response(
        val id: String,
        val success: Boolean,
        val data: ResponseData? = null,
        val error: String? = null
    ) : OutgoingMessage

    @Serializable
    @SerialName("Event")
    data class Event(
        val payload: IPCEvent
    ) : OutgoingMessage
}