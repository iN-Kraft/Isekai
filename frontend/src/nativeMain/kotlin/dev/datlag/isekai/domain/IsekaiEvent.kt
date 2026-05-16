package dev.datlag.isekai.domain

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class IsekaiEvent {
    @Serializable
    @SerialName("progress")
    data class Progress(val step: String, val percent: Int) : IsekaiEvent()

    @Serializable
    @SerialName("fatalError")
    data class FatalError(val message: String) : IsekaiEvent()

    @Serializable
    @SerialName("disksLoaded")
    data class DisksLoaded(val disks: List<Disk>) : IsekaiEvent()
}