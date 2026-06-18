package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.ipc.Disk
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.Partition
import dev.datlag.isekai.repository.DiskRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
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

    private val _hardwareTick = MutableStateFlow(0)
    val hardwareTick = _hardwareTick.asStateFlow()

    init {
        viewModelScope.launch {
            repository.events.collect { event ->
                if (event.message == "HardwareChanged") {
                    loadDisks()
                }
            }
        }
    }

    fun loadDisks() {
        viewModelScope.launch {
            _isLoading.update { true }
            _error.update { null }

            fold(
                block = { repository.getDisks() },
                catch = { e ->
                    _error.update { e.message ?: "An unexpected error occurred" }
                    _isLoading.update { false }
                },
                recover = { err: IPCError ->
                    _error.update { "Failed to load disks: $err" }
                    _isLoading.update { false }
                },
                transform = { loadedDisks ->
                    _disks.update { loadedDisks }
                    _isLoading.update { false }
                    _hardwareTick.update { it + 1 }
                }
            )
        }
    }

    suspend fun loadPartitions(disk: Disk): List<Partition> {
        return fold(
            block = { repository.getPartitions(disk.stableId) },
            catch = { e ->
                e.printStackTrace()
                emptyList()
            },
            recover = { err: IPCError ->
                println(err)
                emptyList()
            },
            transform = { parts ->
                parts
            }
        )
    }
}
