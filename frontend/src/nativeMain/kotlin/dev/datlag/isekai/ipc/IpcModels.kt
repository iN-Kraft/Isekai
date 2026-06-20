package dev.datlag.isekai.ipc

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/**
 * Common IPC message formats.
 */

@Serializable
sealed class IpcRequest {
    abstract val id: String

    @Serializable
    @SerialName("GetState")
    data class GetState(override val id: String) : IpcRequest()

    @Serializable
    @SerialName("CheckSystem")
    data class CheckSystem(override val id: String) : IpcRequest()

    @Serializable
    @SerialName("GetDisks")
    data class GetDisks(override val id: String) : IpcRequest()

    @Serializable
    @SerialName("GetPartitions")
    data class GetPartitions(
        override val id: String,
        @SerialName("disk_id") val diskId: String,
    ) : IpcRequest()

    @Serializable
    @SerialName("UnlockBitlocker")
    data class UnlockBitlocker(
        override val id: String,
        @SerialName("drive_letter") val driveLetter: String
    ) : IpcRequest()

    @Serializable
    @SerialName("SuspendBitlocker")
    data class SuspendBitlocker(
        override val id: String,
        @SerialName("drive_letter") val driveLetter: String
    ) : IpcRequest()

    @Serializable
    @SerialName("ShrinkInstallLocal")
    data class ShrinkInstallLocal(
        override val id: String,
        @SerialName("disk_id") val diskId: String,
        @SerialName("partition_id") val partitionId: String,
        @SerialName("iso_path") val isoPath: String
    ) : IpcRequest()

    @Serializable
    @SerialName("Uninstall")
    data class Uninstall(
        override val id: String,
        @SerialName("disk_id") val diskId: String
    ) : IpcRequest()
}

@Serializable
sealed class OutgoingMessage {

    @Serializable
    @SerialName("Response")
    data class Response(
        val id: String,
        val success: Boolean,
        val data: JsonElement? = null,
        val error: String? = null
    ) : OutgoingMessage()

    @Serializable
    @SerialName("Event")
    data class Event(
        @SerialName("event_type") val eventType: String,
        val message: String,
        val percent: Int? = null
    ) : OutgoingMessage()
}

/**
 * Domain Models mapped from Rust.
 */

@Serializable
enum class WorkflowType {
    @SerialName("ShrinkAndInstall")
    ShrinkAndInstall
}

@Serializable
data class AppState(
    @SerialName("active_workflow") val activeWorkflow: WorkflowType? = null,
    @SerialName("current_step") val currentStep: String? = null,
    @SerialName("step_progress") val stepProgress: Int? = null,
    @SerialName("step_details") val stepDetails: String? = null
)

@Serializable
data class Disk(
    val stableId: String,
    val name: String,
    val totalGb: UInt,
    val freeGb: UInt,
    val isSystemDrive: Boolean
)

@Serializable
data class Partition(
    val id: String,
    val driveLetter: String? = null,
    val sizeGb: UInt,
    val fileSystem: String,
    val bitlockerState: BitLockerState
)

@Serializable
enum class BitLockerState {
    @SerialName("Unprotected") Unprotected,
    @SerialName("Protected") Protected,
    @SerialName("Locked") Locked
}

@Serializable
data class ValidationReport(
    val osName: String,
    val components: List<SystemComponent>,
    val isReady: Boolean
)

@Serializable
data class SystemComponent(
    val name: String,
    val status: ComponentStatus,
    val isCritical: Boolean
)

@Serializable
sealed class ComponentStatus {
    @Serializable
    @SerialName("Installed")
    data class Installed(val version: String) : ComponentStatus()

    @Serializable
    @SerialName("Missing")
    object Missing : ComponentStatus()
}
