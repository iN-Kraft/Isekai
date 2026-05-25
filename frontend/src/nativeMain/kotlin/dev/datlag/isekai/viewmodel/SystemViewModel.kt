package dev.datlag.isekai.viewmodel

import dev.datlag.isekai.ipc.ValidationReport
import dev.datlag.isekai.repository.SystemRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
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
            repository.checkSystem()
                .onSuccess { report ->
                    _systemReport.value = report
                }
                .onFailure { err ->
                    println("Failed to check system: ${err.message}")
                    err.printStackTrace()
                    _error.value = err.message ?: "Unknown error occurred"
                }
        }
    }
}
