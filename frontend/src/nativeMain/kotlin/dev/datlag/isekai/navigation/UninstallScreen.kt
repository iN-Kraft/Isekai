package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.navigation.component.NewComboRow
import dev.datlag.isekai.translation.DistroSelection
import dev.datlag.isekai.viewmodel.DiskViewModel
import dev.datlag.isekai.viewmodel.InstallViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.ResponseAppearance
import dev.datlag.kommons.adwaita.compose.component.ActionRow
import dev.datlag.kommons.adwaita.compose.component.AlertDialog
import dev.datlag.kommons.adwaita.compose.component.AlertResponse
import dev.datlag.kommons.adwaita.compose.component.ButtonRow
import dev.datlag.kommons.adwaita.compose.component.CircularProgressIndicator
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.gtk.StringList
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css

@Composable
fun UninstallScreen(onBack: () -> Unit) = DefaultScreen(DistroSelection) {
    val diskViewModel = kodeinViewModel<DiskViewModel>()
    val installViewModel = kodeinViewModel<InstallViewModel>()

    val state by diskViewModel.state.collectAsState()
    val selectedDiskIndex = remember(state.diskState) {
        state.diskState.selectedIndex ?: 0
    }
    val diskModel = remember(state.diskState.disks) {
        StringList(state.diskState.disks.map { disk ->
            "${disk.name} - ${disk.totalGb}GB"
        })
    }

    PreferencesPage {
        PreferencesGroup {
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
        }
        PreferencesGroup {
            var showConfirm by remember { mutableStateOf(false) }
            AlertDialog(
                visible = showConfirm,
                heading = "Confirm Action",
                body = "Uninstall Isekai Installation",
                responses = listOf(
                    AlertResponse("close", "Close"),
                    AlertResponse("confirm", "Confirm", appearance = ResponseAppearance.DESTRUCTIVE)
                ),
                defaultResponse = "close",
                onClosed = { showConfirm = false },
                onResponse = { response ->
                    if (response.equals("confirm", ignoreCase = true)) {
                        val diskId = state.diskState.selectedDisk?.stableId ?: state.diskState.disks.getOrNull(selectedDiskIndex)?.stableId ?: return@AlertDialog

                        installViewModel.uninstall(diskId)
                    }
                }
            )

            ButtonRow(
                modifier = Modifier.css("suggested-action"),
                title = "Uninstall",
                startIconName = "system-software-install-symbolic",
                onActivated = { showConfirm = true }
            )
            ButtonRow(
                title = "Back",
                startIconName = "system-software-install-symbolic",
                onActivated = { onBack() }
            )
        }
    }
}