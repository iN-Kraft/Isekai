package dev.datlag.isekai.repository

import arrow.core.raise.Raise
import dev.datlag.isekai.common.execute
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCRequest
import dev.datlag.isekai.ipc.IPCTransport

class InstallRepository(private val transport: IPCTransport) {

    context(_: Raise<IPCError>)
    suspend fun shrinkInstallLocal(
        diskId: String,
        partitionId: String,
        isoPath: String
    ): Unit = transport.execute { IPCRequest.ShrinkInstallLocal(it, diskId, partitionId, isoPath) }

    context(_: Raise<IPCError>)
    suspend fun shrinkInstallRemote(
        diskId: String,
        partitionId: String,
        distroId: String
    ): Unit = transport.execute { IPCRequest.ShrinkInstallRemote(it, diskId, partitionId, distroId) }

    context(_: Raise<IPCError>)
    suspend fun uninstall(
        diskId: String
    ): Unit = transport.execute { IPCRequest.Uninstall(it, diskId) }
}