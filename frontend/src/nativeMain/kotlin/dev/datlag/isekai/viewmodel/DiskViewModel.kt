package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCEvent
import dev.datlag.isekai.ipc.model.Disk
import dev.datlag.isekai.ipc.model.Partition
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
                if (event.payload is IPCEvent.SystemHardwareChanged) {
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
                    partitionState = current.partitionState.copy(isLoading = true, selectedLetter = null)
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

    fun selectDisk(index: Int) {
        selectDisk(state.value.diskState.disks.getOrNull(index))
    }

    fun selectPartition(partition: Partition?) {
        if (state.value.partitionState.selectedPartition == partition) {
            return
        }

        _state.update { current ->
            current.copy(
                partitionState = current.partitionState.copy(
                    selectedLetter = partition?.driveLetter,
                    selectedId = partition?.id
                )
            )
        }
    }

    fun selectPartition(index: Int) {
        selectPartition(state.value.partitionState.partitions.getOrNull(index))
    }

    fun unlockBitlocker(partition: Partition? = state.value.partitionState.selectedPartition) {
        val driveLetter = partition?.driveLetter ?: return
        val driveId = partition.id

        viewModelScope.launch {
            fold(
                block = { repository.unlockBitlocker(driveLetter) },
                catch = { e ->
                    e.printStackTrace()
                },
                recover = { err: IPCError ->
                    println(err)
                },
                transform = { }
            )

            val newPartitionState = getPartitionState()
            _state.update { current ->
                current.copy(
                    partitionState = newPartitionState.copy(
                        selectedLetter = driveLetter,
                        selectedId = driveId
                    )
                )
            }
        }
    }

    fun suspendBitlocker(partition: Partition? = state.value.partitionState.selectedPartition) {
        val driveLetter = partition?.driveLetter ?: return
        val driveId = partition.id

        viewModelScope.launch {
            fold(
                block = { repository.suspendBitlocker(driveLetter) },
                catch = { e ->
                    e.printStackTrace()
                },
                recover = { err: IPCError ->
                    println(err)
                },
                transform = { }
            )

            val newPartitionState = getPartitionState()
            _state.update { current ->
                current.copy(
                    partitionState = newPartitionState.copy(
                        selectedLetter = driveLetter,
                        selectedId = driveId
                    )
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
                val validId = current.selectedId?.takeIf { id -> disks.any { it.stableId == id } }
                val newSelectedId = validId ?: disks.firstOrNull()?.stableId

                State.DiskState(
                    isLoading = false,
                    disks = disks,
                    selectedId = newSelectedId
                )
            }
        )
    }

    private suspend fun getPartitionState(
        disk: Disk? = state.value.diskState.selectedDisk,
        current: State.PartitionState = state.value.partitionState
    ): State.PartitionState {
        if (disk == null) {
            return State.PartitionState(
                isLoading = false,
                partitions = emptyList(),
                selectedLetter = null,
                selectedId = null
            )
        }

        return fold(
            block = { repository.getPartitions(disk.stableId) },
            catch = { e ->
                e.printStackTrace()

                State.PartitionState(
                    isLoading = false,
                    partitions = emptyList(),
                    selectedLetter = null,
                    selectedId = null
                )
            },
            recover = { err: IPCError ->
                println(err)

                State.PartitionState(
                    isLoading = false,
                    partitions = emptyList(),
                    selectedLetter = null,
                    selectedId = null
                )
            },
            transform = { parts ->
                val validLetter = current.selectedLetter?.takeIf { letter -> parts.any { it.driveLetter == letter } }
                val validId = if (validLetter != null) {
                    parts.firstOrNull { it.driveLetter == validLetter }?.id
                } else {
                    null
                } ?: current.selectedId?.takeIf { id -> parts.any { it.id == id } }

                State.PartitionState(
                    isLoading = false,
                    partitions = parts,
                    selectedLetter = validLetter,
                    selectedId = validId
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

            val selectedIndex: Int?
                get() = if (selectedId.isNullOrBlank()) {
                    null
                } else {
                    disks.indexOfFirst { it.stableId == selectedId }.takeIf { it >= 0 }
                }
        }

        @Serializable
        data class PartitionState(
            val isLoading: Boolean = true,
            val partitions: List<Partition> = emptyList(),
            val selectedLetter: String? = null,
            val selectedId: String? = null
        ) {
            val selectedPartition: Partition?
                get() = if (selectedLetter.isNullOrBlank()) {
                    if (selectedId.isNullOrBlank()) {
                        null
                    } else {
                        partitions.firstOrNull { it.id == selectedId }
                    }
                } else {
                    partitions.firstOrNull { it.driveLetter == selectedLetter }
                }

            val selectedIndex: Int?
                get() = if (selectedLetter.isNullOrBlank()) {
                    if (selectedId.isNullOrBlank()) {
                        null
                    } else {
                        partitions.indexOfFirst { it.id == selectedId }.takeIf { it >= 0 }
                    }
                } else {
                    partitions.indexOfFirst { it.driveLetter == selectedLetter }.takeIf { it >= 0 }
                }
        }
    }
}
