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
import kotlinx.serialization.Serializable
import org.kodein.di.DirectDI
import org.kodein.di.instance

class InstallViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: InstallRepository = instance()

    private val _state = MutableStateFlow<State>(State.Idle(true))
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
                is IPCEvent.WorkflowStarted -> State.Running.Indeterminate(title = "Starting workflow...")
                is IPCEvent.WorkflowEnded -> {
                    if (event.success) State.Success else State.Error(event.message)
                }

                is IPCEvent.StepInitializingDownload -> State.Running.Downloading(
                    title = "Initialize download...",
                    progress = 0F,
                    etaSeconds = 0uL
                )
                is IPCEvent.ProgressDownload -> State.Running.Downloading(
                    title = "Downloading OS",
                    progress = event.percent.toFloat() / 100F,
                    etaSeconds = event.etaSeconds,
                    downloadedBytes = event.downloadedBytes,
                    totalBytes = event.totalBytes,
                    isPaused = false
                )
                is IPCEvent.StepCopyingPayload -> State.Running.Installing(
                    title = "Writing OS to disk...",
                    progress = 0F
                )
                is IPCEvent.ProgressCopyingPayload -> (current as? State.Running.Installing)?.copy(
                    progress = event.percent.toFloat() / 100F
                ) ?: State.Running.Installing(title = "Writing OS to disk...", progress = event.percent.toFloat() / 100F)

                is IPCEvent.StepMountingISO -> State.Running.Indeterminate(title = "Mounting payload...")
                is IPCEvent.StepCalculatingSpace -> State.Running.Indeterminate(title = "Analyzing disk requirements...")
                is IPCEvent.StepPreFlightChecks -> State.Running.Indeterminate(title = "Running system checks...")
                is IPCEvent.StepShrinkingPartition -> State.Running.Indeterminate(title = "Resizing partition...")
                is IPCEvent.StepCreatingBootPartitions -> State.Running.Indeterminate(title = "Creating boot structures...")
                is IPCEvent.StepConfiguringBootloader -> State.Running.Indeterminate(title = "Patching Boot Manager...")
                is IPCEvent.StepCleaningBootloader -> State.Running.Indeterminate(title = "Removing boot entries...")
                is IPCEvent.StepDeletingPartitions -> State.Running.Indeterminate(title = "Reclaiming disk space...")

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
        val current = _state.value
        if (current is State.Running.Downloading) {
            _state.update { current.copy(isPaused = !current.isPaused) }
            viewModelScope.launch {
                fold(
                    block = { repository.pauseWorkflow() },
                    catch = { e -> e.printStackTrace() },
                    recover = { err: IPCError -> println(err) },
                    transform = { }
                )
            }
        }
    }

    fun cancelWorkflow() {
        if (_state.value !is State.Running) {
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

    @Serializable
    sealed interface State {

        @Serializable
        data class Idle(val initial: Boolean) : State

        @Serializable
        sealed interface Running : State {

            val title: String

            @Serializable
            data class Indeterminate(
                override val title: String
            ) : Running

            @Serializable
            data class Downloading(
                override val title: String,
                val progress: Float,
                val etaSeconds: ULong,
                val downloadedBytes: ULong = 0uL,
                val totalBytes: ULong = downloadedBytes,
                val isPaused: Boolean = false
            ) : Running {

                fun formatETA(): String {
                    if (etaSeconds <= 0uL) {
                        return ""
                    }

                    val hours = etaSeconds / 3600uL
                    val mins = (etaSeconds % 3600uL) / 60uL
                    val secs = etaSeconds % 60uL

                    return buildString {
                        if (hours > 0uL) append("${hours.toString().padStart(2, '0')}:")
                        append("${mins.toString().padStart(2, '0')}:")
                        append(secs.toString().padStart(2, '0'))
                    }.trim()
                }
            }

            @Serializable
            data class Installing(
                override val title: String,
                val progress: Float
            ) : Running
        }

        @Serializable
        data object Success : State

        @Serializable
        data class Error(val message: String?) : State
    }
}