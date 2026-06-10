package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.ValidationReport
import dev.datlag.isekai.repository.SystemRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.kodein.di.DI
import org.kodein.di.DirectDI
import org.kodein.di.instance

class SystemViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: SystemRepository = instance()

    private val _systemReport = MutableStateFlow<ValidationReport?>(null)
    val systemReport = _systemReport.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error = _error.asStateFlow()

    fun checkSystem() {
        viewModelScope.launch {
            _error.value = null

            fold(
                block = { repository.checkSystem() },
                catch = { unexpected ->
                    println("System crashed: ${unexpected.message}")
                    unexpected.printStackTrace()
                    _error.update { "Fatal Error: ${unexpected.message}" }
                },
                recover = { ipcError: IPCError ->
                    println("Failed to check system: $ipcError")
                    _error.update { when (ipcError) {
                        is IPCError.BackendError -> "Daemon rejected request: ${ipcError.message}"
                        is IPCError.Disconnected -> "Daemon disconnected"
                        is IPCError.SerializationError -> "Data parsing failed."
                        is IPCError.ConnectionFailed -> "Connection failed."
                        is IPCError.RequestCancelled -> "Request was cancelled."
                    } }
                },
                transform = { report ->
                    _systemReport.update { report }
                }
            )
        }
    }
}
