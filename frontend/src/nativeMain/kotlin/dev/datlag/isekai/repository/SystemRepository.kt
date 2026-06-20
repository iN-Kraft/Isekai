package dev.datlag.isekai.repository

import arrow.core.raise.Raise
import dev.datlag.isekai.common.execute
import dev.datlag.isekai.ipc.AppState
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCTransport
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.ValidationReport

class SystemRepository(private val transport: IPCTransport) {
    context(_: Raise<IPCError>)
    suspend fun checkSystem(): ValidationReport =
        transport.execute { IpcRequest.CheckSystem(it) }

    context(_: Raise<IPCError>)
    suspend fun getState(): AppState = transport.execute { IpcRequest.GetState(it) }
}
