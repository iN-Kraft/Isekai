package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCTransport
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import org.kodein.di.DI
import org.kodein.di.DirectDI
import org.kodein.di.instance

class ConnectionViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val transport: IPCTransport = instance()

    val connectionState: StateFlow<ConnectionState> = transport.connectionState

    fun connect() {
        viewModelScope.launch {
            fold(
                block = { transport.connect() },
                catch = { e ->
                    println("Unexpected crash during connection: ${e.message}")
                },
                recover = { err: IPCError ->
                    println("Could not establish IPC connection: $err")
                },
                transform = {
                    println("Successfully connected to Isekai Daemon!")
                }
            )
        }
    }

    fun disconnect() {
        transport.disconnect()
    }
}
