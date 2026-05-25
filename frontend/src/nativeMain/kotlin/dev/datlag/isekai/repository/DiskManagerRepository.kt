package dev.datlag.isekai.repository

import dev.datlag.isekai.ipc.Disk
import dev.datlag.isekai.ipc.IpcConnectionException
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.isekai.ipc.OutgoingMessage
import dev.datlag.isekai.ipc.Partition
import dev.datlag.isekai.ipc.ValidationReport
import kotlinx.coroutines.flow.Flow
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.decodeFromJsonElement
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

/**
 * Domain-level repository for disk management operations.
 * Wraps the transport layer and provides a clean, typed API for ViewModels.
 */
class DiskManagerRepository(private val transport: IpcTransport) {

    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Exposes IPC events as a typed Flow.
     */
    val events: Flow<OutgoingMessage.Event> = transport.events

    /**
     * Performs a system-wide validation check.
     */
    suspend fun checkSystem(): Result<ValidationReport> =
        execute { IpcRequest.CheckSystem(it) }

    /**
     * Retrieves a list of available disks.
     */
    suspend fun getDisks(): Result<List<Disk>> =
        execute { IpcRequest.GetDisks(it) }

    /**
     * Retrieves partitions for a specific disk.
     */
    suspend fun getPartitions(diskId: String): Result<List<Partition>> =
        execute { IpcRequest.GetPartitions(it, diskId) }

    /**
     * Requests a partition shrink operation.
     */
    suspend fun shrinkPartition(diskId: String, partitionId: String, targetSizeGb: UInt): Result<Unit> =
        execute<Unit> { IpcRequest.ShrinkPartition(it, diskId, partitionId, targetSizeGb) }

    /**
     * Internal helper to execute a request, handle IDs, and decode the response.
     */
    private suspend inline fun <reified T> execute(
        requestFactory: (String) -> IpcRequest
    ): Result<T> {
        return try {
            val id = generateId()
            val request = requestFactory(id)
            val response = transport.send(request)

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

    /**
     * Generates a random unique ID for the request.
     */
    @OptIn(ExperimentalUuidApi::class)
    private fun generateId(): String {
        return Uuid.random().toString()
    }
}