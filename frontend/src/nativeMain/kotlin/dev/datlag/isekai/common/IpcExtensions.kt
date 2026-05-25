package dev.datlag.isekai.common

import dev.datlag.isekai.ipc.IpcConnectionException
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.IpcTransport
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.decodeFromJsonElement
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

private val json = Json { ignoreUnknownKeys = true }

@OptIn(ExperimentalUuidApi::class)
internal suspend inline fun <reified T> IpcTransport.execute(
    requestFactory: (String) -> IpcRequest
): Result<T> {
    return try {
        val id = Uuid.random().toString()
        val request = requestFactory(id)
        val response = send(request)

        if (response.success) {
            if (T::class == Unit::class) {
                @Suppress("UNCHECKED_CAST")
                Result.success(Unit as T)
            } else {
                val data = response.data ?: return Result.failure(
                    IpcConnectionException("Operation succeeded but returned no data")
                )
                val decoded = json.decodeFromJsonElement<T>(data)
                Result.success(decoded)
            }
        } else {
            Result.failure(Exception(response.error ?: "Unknown backend error"))
        }
    } catch (e: Exception) {
        Result.failure(e)
    }
}
