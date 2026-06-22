package dev.datlag.isekai.ipc.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
enum class WorkflowType {
    @SerialName("shrink_and_install")
    ShrinkAndInstall,

    @SerialName("download_iso")
    DownloadIso,

    @SerialName("wipe_disk")
    WipeDisk,

    @SerialName("uninstall")
    Uninstall
}