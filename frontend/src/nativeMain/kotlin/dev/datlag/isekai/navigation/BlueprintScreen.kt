package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.ipc.model.BitlockerState
import dev.datlag.isekai.navigation.component.NewComboRow
import dev.datlag.isekai.viewmodel.DiskViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.BannerButtonStyle
import dev.datlag.kommons.adwaita.ResponseAppearance
import dev.datlag.kommons.adwaita.compose.component.ActionRow
import dev.datlag.kommons.adwaita.compose.component.AlertDialog
import dev.datlag.kommons.adwaita.compose.component.AlertResponse
import dev.datlag.kommons.adwaita.compose.component.Banner
import dev.datlag.kommons.adwaita.compose.component.ButtonContent
import dev.datlag.kommons.adwaita.compose.component.ButtonRow
import dev.datlag.kommons.adwaita.compose.component.CircularProgressIndicator
import dev.datlag.kommons.adwaita.compose.component.Clamp
import dev.datlag.kommons.adwaita.compose.component.ComboRow
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.SpinRow
import dev.datlag.kommons.adwaita.compose.component.SwitchRow
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.Align
import dev.datlag.kommons.gtk.StringList
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.component.Image
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.alignHorizontal
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import dev.datlag.kommons.gtk.glib.GLib
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun BlueprintScreen(
    config: Screen.BlueprintScreen,
    onBack: () -> Unit,
    onInstall: (Screen.Install) -> Unit
) {
    val diskViewModel = kodeinViewModel<DiskViewModel>(dispatcher = Dispatchers.IO)
    val state by diskViewModel.state.collectAsState()

    val selectedDiskIndex = remember(state.diskState) {
        state.diskState.selectedIndex ?: 0
    }

    val selectedPartitionIndex = remember(state.partitionState.partitions, state.partitionState.selectedLetter) {
        state.partitionState.selectedIndex ?: 0
    }
    val bitlockerState = remember(state.partitionState.partitions, selectedPartitionIndex) {
        state.partitionState.partitions.getOrNull(selectedPartitionIndex)?.bitlockerState ?: BitlockerState.Unprotected
    }
    var isBitLockerActive by remember(bitlockerState) {
        mutableStateOf(bitlockerState != BitlockerState.Unprotected)
    }

    val diskModel = remember(state.diskState.disks) {
        StringList(state.diskState.disks.map { disk ->
            "${disk.name} - ${disk.totalGb}GB"
        })
    }

    val partitionModel = remember(state.partitionState.partitions) {
        StringList(state.partitionState.partitions.map { part ->
            val driveDisplay = part.driveLetter ?: "Volume"
            "$driveDisplay (${part.fileSystem}) - ${part.sizeGb}GB"
        })
    }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                modifier = Modifier.fillMaxWidth(),
                navigationIcon = {
                    Button(
                        modifier = Modifier.css("flat", "circular"),
                        onClick = onBack
                    ) {
                        Image(iconName = IconName("go-previous-symbolic"))
                    }
                },
                title = { WindowTitle("Prepare Installation") }
            )

            Banner(
                title = when (bitlockerState) {
                    BitlockerState.Locked -> "Drive is locked with BitLocker."
                    BitlockerState.Protected -> "Drive is protected on reboot."
                    BitlockerState.Unprotected -> ""
                },
                revealed = isBitLockerActive,
                buttonLabel = when (bitlockerState) {
                    BitlockerState.Locked -> "Unlock"
                    BitlockerState.Protected -> "Suspend"
                    BitlockerState.Unprotected -> null
                },
                buttonStyle = BannerButtonStyle.SUGGESTED,
                onButtonClicked = {
                    when (bitlockerState) {
                        BitlockerState.Locked -> diskViewModel.unlockBitlocker()
                        BitlockerState.Protected -> diskViewModel.suspendBitlocker()
                        else -> { }
                    }
                }
            )
        }
    ) {
        Clamp(modifier = Modifier.fillMaxSize(), maximumSize = 800) {
            PreferencesPage(
                modifier = Modifier.fillMaxSize()
            ) {
                PreferencesGroup(
                    title = "Destination Drive",
                    description = "This will be resized to make room for Linux."
                ) {
                    var wipeDisk by remember(selectedDiskIndex, state.diskState.selectedDisk) {
                        mutableStateOf(false)
                    }
                    var additionalSpace by remember { mutableDoubleStateOf(0.0) }

                    if (state.diskState.isLoading) {
                        ActionRow(
                            title = "Target Drive",
                            subtitle = "Checking for available drives",
                            suffix = { CircularProgressIndicator() }
                        )
                    }

                    NewComboRow(
                        title = "Select Target Drive",
                        useSubtitle = true,
                        model = diskModel,
                        selected = selectedDiskIndex,
                        onSelectedChange = {
                            diskViewModel.selectDisk(it)
                        },
                        enableSearch = false,
                        visible = !state.diskState.isLoading
                    )
                    SwitchRow(
                        title = "Wipe Disk",
                        subtitle = "Clear entire drive",
                        active = wipeDisk,
                        enabled = state.diskState.selectedDisk?.isSystemDrive?.not() ?: true,
                        onActiveChanged = { wipeDisk = it }
                    )

                    if (!wipeDisk) {
                        if (state.partitionState.isLoading) {
                            ActionRow(
                                title = "Partition",
                                subtitle = "Checking for available partitions",
                                suffix = { CircularProgressIndicator() }
                            )
                        }
                    }
                    NewComboRow(
                        title = "Partition",
                        useSubtitle = true,
                        model = partitionModel,
                        selected = selectedPartitionIndex,
                        onSelectedChange = {
                            diskViewModel.selectPartition(it)
                        },
                        enableSearch = false,
                        visible = !wipeDisk && !state.partitionState.isLoading
                    )

                    SpinRow(
                        value = additionalSpace,
                        onValueChange = {
                            additionalSpace = it
                        },
                        title = "Additional Space",
                        subtitle = "This space can be used to actually install the Linux Distribution then.",
                        max = state.diskState.selectedDisk?.freeGb?.takeIf { it > 0u }?.toDouble() ?: 100.0
                    )
                }

                PreferencesGroup(
                    title = "System",
                    enabled = !isBitLockerActive
                ) {
                    var delete by remember { mutableStateOf(false) }

                    ActionRow(
                        prefix = {
                            Image(iconName = IconName("computer-symbolic"))
                        },
                        title = "Selected System",
                        subtitle = when (config) {
                            is Screen.BlueprintScreen.LocalFile -> {
                                "${config.filePath} - ${GLib.formatSize(config.fileSize)}"
                            }
                            is Screen.BlueprintScreen.Download -> {
                                if (config.edition.isNullOrBlank()) {
                                    config.name
                                } else {
                                    "${config.name} - ${config.edition}"
                                }
                            }
                        },
                    )
                    if (config is Screen.BlueprintScreen.Download) {
                        SwitchRow(
                            title = "Delete ISO",
                            subtitle = "Automatically delete the downloaded ISO after installation?",
                            active = delete,
                            onActiveChanged = { delete = it }
                        )
                    }
                }

                PreferencesGroup(
                    enabled = !isBitLockerActive
                ) {
                    var showConfirm by remember { mutableStateOf(false) }
                    AlertDialog(
                        visible = showConfirm,
                        heading = "Confirm Action",
                        body = "This action will be run in background. Even if you close the application or in case it crashes, the task will finish to prevent data corruption.",
                        responses = listOf(
                            AlertResponse("close", "Close"),
                            AlertResponse("confirm", "Confirm", appearance = ResponseAppearance.DESTRUCTIVE)
                        ),
                        defaultResponse = "close",
                        onClosed = { showConfirm = false },
                        onResponse = { response ->
                            if (response.equals("confirm", ignoreCase = true)) {
                                when (config) {
                                    is Screen.BlueprintScreen.LocalFile -> {
                                        val diskId = state.diskState.selectedDisk?.stableId ?: state.diskState.disks.getOrNull(selectedDiskIndex)?.stableId ?: return@AlertDialog
                                        val partitionId = state.partitionState.selectedPartition?.id ?: state.partitionState.partitions.getOrNull(selectedPartitionIndex)?.id ?: return@AlertDialog

                                        onInstall(Screen.Install.Shrink.Local(
                                            diskId = diskId,
                                            partitionId = partitionId,
                                            filePath = config.filePath
                                        ))
                                    }
                                    is Screen.BlueprintScreen.Download -> {
                                        val diskId = state.diskState.selectedDisk?.stableId ?: state.diskState.disks.getOrNull(selectedDiskIndex)?.stableId ?: return@AlertDialog
                                        val partitionId = state.partitionState.selectedPartition?.id ?: state.partitionState.partitions.getOrNull(selectedPartitionIndex)?.id ?: return@AlertDialog

                                        onInstall(Screen.Install.Shrink.Remote(
                                            diskId = diskId,
                                            partitionId = partitionId,
                                            distroId = config.id
                                        ))
                                    }
                                }
                            }
                        }
                    )

                    ButtonRow(
                        modifier = Modifier.css("suggested-action"),
                        title = when (config) {
                            is Screen.BlueprintScreen.Download -> "Download &amp; Install"
                            is Screen.BlueprintScreen.LocalFile -> "Install"
                        },
                        startIconName = "system-software-install-symbolic",
                        onActivated = { showConfirm = true }
                    )
                }
            }
        }
    }
}