package dev.datlag.isekai.ipc.model

import kotlinx.serialization.Serializable

@Serializable
data class Disk(
    val stableId: String,
    val name: String,
    val totalGb: UInt,
    val freeGb: UInt,
    val isSystemDrive: Boolean,
    val isGpt: Boolean
)