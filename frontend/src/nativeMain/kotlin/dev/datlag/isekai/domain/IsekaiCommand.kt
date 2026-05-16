package dev.datlag.isekai.domain

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class IsekaiCommand {
    @Serializable
    @SerialName("getDisks")
    data object GetDisks : IsekaiCommand()

    @Serializable
    @SerialName("shrinkPartition")
    data class ShrinkPartition(val diskNum: Int, val sizeGb: Int) : IsekaiCommand()
}