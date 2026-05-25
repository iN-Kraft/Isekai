package dev.datlag.isekai.repository

import dev.datlag.isekai.common.execute
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.isekai.ipc.ValidationReport

class SystemRepository(private val transport: IpcTransport) {
    suspend fun checkSystem(): Result<ValidationReport> =
        transport.execute { IpcRequest.CheckSystem(it) }
}
