package dev.datlag.isekai.repository

import dev.datlag.isekai.common.execute
import dev.datlag.isekai.ipc.Disk
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.isekai.ipc.OutgoingMessage
import dev.datlag.isekai.ipc.Partition
import kotlinx.coroutines.flow.Flow

class DiskRepository(private val transport: IpcTransport) {
    val events: Flow<OutgoingMessage.Event> = transport.events

    suspend fun getDisks(): Result<List<Disk>> =
        transport.execute { IpcRequest.GetDisks(it) }

    suspend fun getPartitions(diskId: String): Result<List<Partition>> =
        transport.execute { IpcRequest.GetPartitions(it, diskId) }

    suspend fun shrinkPartition(diskId: String, partitionId: String, targetSizeGb: UInt): Result<Unit> =
        transport.execute<Unit> { IpcRequest.ShrinkPartition(it, diskId, partitionId, targetSizeGb) }
}
