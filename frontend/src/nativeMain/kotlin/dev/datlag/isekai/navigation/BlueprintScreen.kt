package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.ipc.BitLockerState
import dev.datlag.isekai.ipc.Partition
import dev.datlag.isekai.viewmodel.DiskViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.BannerButtonStyle
import dev.datlag.kommons.adwaita.compose.component.ActionRow
import dev.datlag.kommons.adwaita.compose.component.Banner
import dev.datlag.kommons.adwaita.compose.component.CircularProgressIndicator
import dev.datlag.kommons.adwaita.compose.component.Clamp
import dev.datlag.kommons.adwaita.compose.component.ComboRow
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.SwitchRow
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.StringList
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.component.Image
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun BlueprintScreen(
    onBack: () -> Unit
) {
    val diskViewModel = kodeinViewModel<DiskViewModel>(dispatcher = Dispatchers.IO)
    val disks by diskViewModel.disks.collectAsState()
    var selectedDiskIndex by remember { mutableStateOf(0) }
    var partitions by remember(disks, selectedDiskIndex) {
        mutableStateOf(emptyList<Partition>())
    }
    var selectedPartitionIndex by remember(partitions) {
        mutableStateOf(0)
    }
    var isBitLockerActive by remember(partitions, selectedPartitionIndex) {
        mutableStateOf(partitions.getOrNull(selectedPartitionIndex)?.let {
            it.bitlockerState != BitLockerState.Unprotected
        } ?: false)
    }

    LaunchedEffect(Unit) {
        diskViewModel.loadDisks()
    }

    LaunchedEffect(disks, selectedDiskIndex) {
        partitions = disks.getOrNull(selectedDiskIndex)?.let { diskViewModel.loadPartitions(it) }.orEmpty()
    }

    val diskModel = remember(disks) {
        StringList(disks.map { disk ->
            "${disk.name} - ${disk.totalGb}GB"
        })
    }

    val partitionModel = remember(partitions) {
        StringList(partitions.map { part ->
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
                title = "BitLocker encryption detected on selected drive.",
                revealed = isBitLockerActive,
                buttonLabel = "Unlock Drive",
                buttonStyle = BannerButtonStyle.SUGGESTED,
                onButtonClicked = {
                    isBitLockerActive = false
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
                    if (disks.isEmpty()) {
                        ActionRow(
                            title = "Target Drive",
                            subtitle = "Checking for available drives",
                            suffix = { CircularProgressIndicator() }
                        )
                    } else {
                        ComboRow(
                            title = "Select Target Drive",
                            useSubtitle = true,
                            model = diskModel,
                            selected = selectedDiskIndex,
                            onSelectedChange = {
                                selectedDiskIndex = it
                            },
                            enableSearch = false
                        )
                    }
                    if (partitions.isEmpty()) {
                        ActionRow(
                            title = "Partition",
                            subtitle = "Checking for available partitions",
                            suffix = { CircularProgressIndicator() }
                        )
                    } else {
                        ComboRow(
                            title = "Partition",
                            useSubtitle = true,
                            model = partitionModel,
                            selected = selectedPartitionIndex,
                            onSelectedChange = {
                                selectedPartitionIndex = it
                            },
                            enableSearch = false
                        )
                    }
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
                        subtitle = "Fedora",
                    )
                    SwitchRow(
                        title = "Delete ISO",
                        subtitle = "Automatically delete the downloaded ISO after installation?",
                        active = delete,
                        onActiveChanged = { delete = it }
                    )
                }
            }
        }
    }
}