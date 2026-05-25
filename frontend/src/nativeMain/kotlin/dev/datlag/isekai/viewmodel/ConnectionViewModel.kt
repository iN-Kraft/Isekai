package dev.datlag.isekai.viewmodel

import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.ipc.IpcTransport
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

    private val transport: IpcTransport = instance()

    val connectionState: StateFlow<ConnectionState> = transport.connectionState

    fun connect() {
        viewModelScope.launch {
            transport.connect()
        }
    }

    fun disconnect() {
        transport.disconnect()
    }
}
