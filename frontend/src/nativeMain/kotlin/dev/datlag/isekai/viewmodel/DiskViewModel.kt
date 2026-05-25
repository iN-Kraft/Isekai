package dev.datlag.isekai.viewmodel

import dev.datlag.isekai.ipc.Disk
import dev.datlag.isekai.repository.DiskRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.kodein.di.DI
import org.kodein.di.DirectDI
import org.kodein.di.instance

class DiskViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: DiskRepository = instance()

    private val _disks = MutableStateFlow<List<Disk>>(emptyList())
    val disks = _disks.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error = _error.asStateFlow()

    fun loadDisks() {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            repository.getDisks()
                .onSuccess { 
                    _disks.value = it 
                    _isLoading.value = false
                }
                .onFailure { 
                    _error.value = it.message ?: "Failed to load disks"
                    _isLoading.value = false
                }
        }
    }
}
