package dev.datlag.isekai.repository

import arrow.core.raise.Raise
import dev.datlag.isekai.common.execute
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCRequest
import dev.datlag.isekai.ipc.IPCTransport
import dev.datlag.isekai.navigation.model.DistroList
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class DistroRepository(private val ipc: IPCTransport) {
    private val mutex = Mutex()

    context(_: Raise<IPCError>)
    suspend fun getDistroInfo(): Map<String, DistroList.PublicConfig> {
        return mutex.withLock {
            ipc.execute<Map<String, DistroList.PublicConfig>> {
                IPCRequest.GetDistroInfo(it)
            }
        }
    }
}