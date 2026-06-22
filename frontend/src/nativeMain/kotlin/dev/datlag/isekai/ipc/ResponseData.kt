package dev.datlag.isekai.ipc

import dev.datlag.isekai.ipc.model.Disk
import dev.datlag.isekai.ipc.model.Partition
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed interface ResponseData {

    @Serializable
    @SerialName("Disks")
    data class Disks(val payload: List<Disk>) : ResponseData

    @Serializable
    @SerialName("Partitions")
    data class Partitions(val payload: List<Partition>) : ResponseData

    @Serializable
    @SerialName("Empty")
    data object Empty : ResponseData
}