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
import kotlinx.coroutines.flow.updateAndGet
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.Transient
import org.kodein.di.DI
import org.kodein.di.DirectDI
import org.kodein.di.instance

class DiskViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: DiskRepository = instance()

    private val _state = MutableStateFlow<State>(State())
    val state = _state.asStateFlow()

    init {
        loadData()

        viewModelScope.launch {
            repository.events.collect { event ->
                if (event.message == "HardwareChanged") {
                    loadData()
                }
            }
        }
    }

    private fun loadData() {
        viewModelScope.launch {
            val currentLoading = _state.updateAndGet { current ->
                current.copy(
                    diskState = current.diskState.copy(isLoading = true),
                    partitionState = current.partitionState.copy(isLoading = true)
                )
            }

            val newDiskState = getDiskState(currentLoading.diskState)
            val newPartitionState = getPartitionState(newDiskState.selectedDisk, currentLoading.partitionState)

            _state.update { current ->
                current.copy(
                    diskState = newDiskState,
                    partitionState = newPartitionState
                )
            }
        }
    }

    fun selectDisk(disk: Disk?) {
        if (state.value.diskState.selectedId == disk?.stableId) {
            return
        }

        viewModelScope.launch {
            val currentLoading = _state.updateAndGet { current ->
                current.copy(
                    diskState = current.diskState.copy(selectedId = disk?.stableId),
                    partitionState = current.partitionState.copy(isLoading = true, selectedId = null)
                )
            }

            val newPartitionState = getPartitionState(disk, currentLoading.partitionState)
            _state.update { current ->
                current.copy(
                    partitionState = newPartitionState
                )
            }
        }
    }

    private suspend fun getDiskState(current: State.DiskState = state.value.diskState): State.DiskState {
        return fold(
            block = { repository.getDisks() },
            catch = { e ->
                e.printStackTrace()

                State.DiskState(
                    isLoading = false,
                    disks = emptyList(),
                    selectedId = null
                )
            },
            recover = { err: IPCError ->
                println(err)

                State.DiskState(
                    isLoading = false,
                    disks = emptyList(),
                    selectedId = null
                )
            },
            transform = { disks ->
                State.DiskState(
                    isLoading = false,
                    disks = disks,
                    selectedId = current.selectedId?.ifBlank { null }?.let {
                        if (disks.any { d -> d.stableId == it }) {
                            it
                        } else {
                            null
                        }
                    }
                )
            }
        )
    }

    private suspend fun getPartitionState(disk: Disk?, current: State.PartitionState): State.PartitionState {
        if (disk == null) {
            return State.PartitionState(
                isLoading = true,
                partitions = emptyList()
            )
        }

        return fold(
            block = { repository.getPartitions(disk.stableId) },
            catch = { e ->
                e.printStackTrace()

                State.PartitionState(
                    isLoading = false,
                    partitions = emptyList(),
                    selectedId = null
                )
            },
            recover = { err: IPCError ->
                println(err)

                State.PartitionState(
                    isLoading = false,
                    partitions = emptyList(),
                    selectedId = null
                )
            },
            transform = { parts ->
                State.PartitionState(
                    isLoading = false,
                    partitions = parts,
                    selectedId = current.selectedId?.ifBlank { null }?.let {
                        if (parts.any { p -> p.id == it }) {
                            it
                        } else {
                            null
                        }
                    }
                )
            }
        )
    }

    @Serializable
    data class State(
        val diskState: DiskState = DiskState(),
        val partitionState: PartitionState = PartitionState()
    ) {
        @Serializable
        data class DiskState(
            val isLoading: Boolean = true,
            val disks: List<Disk> = emptyList(),
            val selectedId: String? = null
        ) {
            val selectedDisk: Disk?
                get() = if (selectedId.isNullOrBlank()) {
                    null
                } else {
                    disks.firstOrNull { it.stableId == selectedId }
                }
        }

        @Serializable
        data class PartitionState(
            val isLoading: Boolean = true,
            val partitions: List<Partition> = emptyList(),
            val selectedId: String? = null
        )
    }
}
