package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.ipc.IPCEvent
import dev.datlag.isekai.repository.InstallRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.kodein.di.DirectDI
import org.kodein.di.instance

class InstallViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: InstallRepository = instance()

    private val _state = MutableStateFlow(State(title = "Preparing..."))
    val state = _state.asStateFlow()

    init {
        viewModelScope.launch {
            repository.events.collect { event ->
                handleIncomingEvent(event.payload)
            }
        }
    }

    private fun handleIncomingEvent(event: IPCEvent) {
        _state.update { current ->
            when (event) {
                is IPCEvent.WorkflowStarted -> current.copy(isRunning = true, title = "Starting workflow...")
                is IPCEvent.WorkflowEnded -> current.copy(
                    isRunning = false,
                    isFinished = true,
                    isDownloading = false,
                    progress = if (event.success) 1F else current.progress,
                    title = if (event.success) "Complete!" else "Failed!"
                )

                is IPCEvent.StepInitializingDownload -> current.copy(
                    isDownloading = true,
                    isPaused = false,
                    progress = null,
                    title = "Initialize download..."
                )
                is IPCEvent.ProgressDownload -> current.copy(
                    isDownloading = true,
                    isPaused = false,
                    progress = event.percent.toFloat() / 100F,
                    title = "Downloading OS"
                )

                is IPCEvent.StepMountingISO -> current.copy(
                    isDownloading = false,
                    progress = null,
                    title = "Mounting payload..."
                )
                is IPCEvent.StepCalculatingSpace -> current.copy(progress = null, title = "Analyzing disk requirements...")
                is IPCEvent.StepPreFlightChecks -> current.copy(progress = null, title = "Running system checks...")
                is IPCEvent.StepShrinkingPartition -> current.copy(progress = null, title = "Resizing partition...")
                is IPCEvent.StepCreatingBootPartitions -> current.copy(progress = null, title = "Creating boot structures...")
                is IPCEvent.StepCopyingPayload -> current.copy(progress = null, title = "Writing OS to disk...")
                is IPCEvent.ProgressCopyingPayload -> current.copy(
                    progress = event.percent.toFloat() / 100F,
                )
                is IPCEvent.StepConfiguringBootloader -> current.copy(progress = null, title = "Patching Boot Manager...")

                is IPCEvent.StepCleaningBootloader -> current.copy(progress = null, title = "Removing boot entries...")
                is IPCEvent.StepDeletingPartitions -> current.copy(progress = null, title = "Reclaiming disk space...")

                else -> current
            }
        }
    }

    fun shrinkInstallLocal(diskId: String, partitionId: String, isoPath: String) {
        viewModelScope.launch {
            fold(
                block = { repository.shrinkInstallLocal(diskId, partitionId, isoPath) },
                catch = { e -> e.printStackTrace() },
                recover = { err: IPCError -> println(err) },
                transform = { }
            )
        }
    }

    fun shrinkInstallRemote(diskId: String, partitionId: String, distroId: String) {
        viewModelScope.launch {
            fold(
                block = { repository.shrinkInstallRemote(diskId, partitionId, distroId) },
                catch = { e -> e.printStackTrace() },
                recover = { err: IPCError -> println(err) },
                transform = { }
            )
        }
    }

    fun uninstall(diskId: String) {
        viewModelScope.launch {
            fold(
                block = { repository.uninstall(diskId) },
                catch = { e -> e.printStackTrace() },
                recover = { err: IPCError -> println(err) },
                transform = { }
            )
        }
    }

    fun togglePause() {
        _state.update { it.copy(isPaused = !it.isPaused) }
        viewModelScope.launch {
            fold(
                block = { repository.pauseWorkflow() },
                catch = { e -> e.printStackTrace() },
                recover = { err: IPCError -> println(err) },
                transform = { }
            )
        }
    }

    fun cancelWorkflow() {
        if (!_state.value.isRunning) {
            return
        }

        viewModelScope.launch {
            fold(
                block = { repository.cancelWorkflow() },
                catch = { e -> e.printStackTrace() },
                recover = { err: IPCError -> println(err) },
                transform = { }
            )
        }
    }

    data class State(
        val isRunning: Boolean = false,
        val isFinished: Boolean = false,
        val isDownloading: Boolean = false,
        val isPaused: Boolean = false,
        val progress: Float? = null,
        val title: String
    )
}