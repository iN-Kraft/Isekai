package dev.datlag.isekai.ipc

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed interface IPCRequest {
    val id: String

    @Serializable
    @SerialName("GetDisks")
    data class GetDisks(override val id: String) : IPCRequest

    @Serializable
    @SerialName("GetPartitions")
    data class GetPartitions(
        override val id: String,
        @SerialName("disk_id") val diskId: String
    ) : IPCRequest

    @Serializable
    @SerialName("UnlockBitlocker")
    data class UnlockBitlocker(
        override val id: String,
        @SerialName("drive_letter") val driveLetter: String
    ) : IPCRequest

    @Serializable
    @SerialName("SuspendBitlocker")
    data class SuspendBitlocker(
        override val id: String,
        @SerialName("drive_letter") val driveLetter: String
    ) : IPCRequest

    @Serializable
    @SerialName("GetDistroInfo")
    data class GetDistroInfo(
        override val id: String
    ) : IPCRequest

    @Serializable
    @SerialName("ShrinkInstallLocal")
    data class ShrinkInstallLocal(
        override val id: String,
        @SerialName("disk_id") val diskId: String,
        @SerialName("partition_id") val partitionId: String,
        @SerialName("iso_path") val isoPath: String
    ) : IPCRequest

    @Serializable
    @SerialName("Uninstall")
    data class Uninstall(
        override val id: String,
        @SerialName("disk_id") val diskId: String
    ) : IPCRequest
}