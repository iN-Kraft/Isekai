package dev.datlag.isekai.domain

import kotlinx.serialization.Serializable

@Serializable
data class Disk(
    val diskNum: Int,
    val name: String,
    val totalGb: Int,
    val freeGb: Int,
    val isSystemDrive: Boolean
)
