package dev.datlag.isekai.common

import arrow.core.raise.Raise
import arrow.core.raise.context.ensureNotNull
import arrow.core.raise.context.raise
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCRequest
import dev.datlag.isekai.ipc.IPCTransport
import dev.datlag.isekai.ipc.ResponseData
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.decodeFromJsonElement
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

private val json = Json { ignoreUnknownKeys = true }

@OptIn(ExperimentalUuidApi::class)
context(_: Raise<IPCError>)
internal suspend inline fun <reified T> IPCTransport.execute(
    requestFactory: (String) -> IPCRequest
): T {
    val id = Uuid.random().toString()
    val request = requestFactory(id)
    val response = send(request)

    if (T::class == Unit::class) {
        @Suppress("UNCHECKED_CAST")
        return Unit as T
    }

    val data = ensureNotNull(response.data) {
        IPCError.SerializationError("Operation succeeded but returned no data for ${T::class.simpleName}")
    }

    return when (data) {
        is ResponseData.Disks -> data.payload as? T
        is ResponseData.Partitions -> data.payload as? T
        is ResponseData.Empty -> Unit as? T
    } ?: raise(IPCError.SerializationError("Failed to map ResponseData to ${T::class.simpleName}"))
}
