package dev.datlag.isekai.ipc.model

import kotlinx.serialization.Serializable

@Serializable
data class Partition(
    val id: String,
    val driveLetter: String? = null,
    val sizeGb: UInt,
    val fileSystem: String,
    val sizeBytes: ULong,
    val freeBytes: ULong,
    val bitlockerState: BitlockerState
)