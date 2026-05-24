package dev.datlag.isekai.ipc

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
sealed class OutgoingMessage {

    @Serializable
    @SerialName("Response")
    data class Response(
        val id: String,
        val success: Boolean,
        val data: JsonElement? = null,
        val error: String? = null
    ) : OutgoingMessage()

    @Serializable
    @SerialName("Event")
    data class Event(
        @SerialName("event_type") val eventType: String,
        val message: String,
        val percent: Int? = null
    ) : OutgoingMessage()
}
