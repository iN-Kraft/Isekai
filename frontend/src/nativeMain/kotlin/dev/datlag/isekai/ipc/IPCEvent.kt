package dev.datlag.isekai.ipc

import dev.datlag.isekai.ipc.model.WorkflowType
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed interface IPCEvent {

    @Serializable
    @SerialName("WorkflowStarted")
    data class WorkflowStarted(
        @SerialName("workflow_type") val workflowType: WorkflowType
    ) : IPCEvent

    @Serializable
    @SerialName("WorkflowEnded")
    data class WorkflowEnded(
        val success: Boolean,
        val message: String? = null
    ) : IPCEvent

    // --- Uninstall Steps ---
    @Serializable
    @SerialName("StepCleaningBootloader")
    data object StepCleaningBootloader : IPCEvent

    @Serializable
    @SerialName("StepDeletingPartitions")
    data object StepDeletingPartitions : IPCEvent

    // --- Install Steps ---
    @Serializable
    @SerialName("StepMountingISO")
    data object StepMountingISO : IPCEvent

    @Serializable
    @SerialName("StepCalculatingSpace")
    data object StepCalculatingSpace : IPCEvent

    @Serializable
    @SerialName("StepPreFlightChecks")
    data object StepPreFlightChecks : IPCEvent

    @Serializable
    @SerialName("StepShrinkingPartition")
    data class StepShrinkingPartition(
        @SerialName("partition_id") val partitionId: String
    ) : IPCEvent

    @Serializable
    @SerialName("StepCreatingBootPartitions")
    data object StepCreatingBootPartitions : IPCEvent

    @Serializable
    @SerialName("StepCopyingPayload")
    data object StepCopyingPayload : IPCEvent

    @Serializable
    @SerialName("StepConfiguringBootloader")
    data object StepConfiguringBootloader : IPCEvent

    @Serializable
    @SerialName("ProgressCopyingPayload")
    data class ProgressCopyingPayload(
        @SerialName("copied_bytes") val copiedBytes: ULong,
        @SerialName("total_bytes") val totalBytes: ULong,
        @SerialName("percent") val percent: UByte
    ) : IPCEvent

    // --- Alerts & Hardware ---
    @Serializable
    @SerialName("Warning")
    data class Warning(val message: String) : IPCEvent

    @Serializable
    @SerialName("SystemHardwareChanged")
    data object SystemHardwareChanged : IPCEvent
}