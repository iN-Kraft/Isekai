package dev.datlag.isekai.repository

import arrow.core.raise.Raise
import dev.datlag.isekai.common.execute
import dev.datlag.isekai.ipc.Disk
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCTransport
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.OutgoingMessage
import dev.datlag.isekai.ipc.Partition
import kotlinx.coroutines.flow.Flow

class DiskRepository(private val transport: IPCTransport) {
    val events: Flow<OutgoingMessage.Event> = transport.events

    context(_: Raise<IPCError>)
    suspend fun getDisks(): List<Disk> =
        transport.execute { IpcRequest.GetDisks(it) }

    context(_: Raise<IPCError>)
    suspend fun getPartitions(diskId: String): List<Partition> =
        transport.execute { IpcRequest.GetPartitions(it, diskId) }

    context(_: Raise<IPCError>)
    suspend fun unlockBitlocker(driveLetter: String): Unit =
        transport.execute { IpcRequest.UnlockBitlocker(it, driveLetter) }

    context(_: Raise<IPCError>)
    suspend fun suspendBitlocker(driveLetter: String): Unit =
        transport.execute { IpcRequest.SuspendBitlocker(it, driveLetter) }
}
