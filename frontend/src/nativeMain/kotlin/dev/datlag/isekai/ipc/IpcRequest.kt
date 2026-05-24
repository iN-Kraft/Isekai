package dev.datlag.isekai.ipc

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class IpcRequest {
    abstract val id: String

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
    @SerialName("ShrinkPartition")
    data class ShrinkPartition(
        override val id: String,
        @SerialName("disk_id") val diskId: String,
        @SerialName("partition_id") val partitionId: String,
        @SerialName("target_size_gb") val targetSizeGb: UInt,
    ) : IpcRequest()
}
