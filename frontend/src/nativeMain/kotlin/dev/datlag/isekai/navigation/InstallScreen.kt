package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import dev.datlag.isekai.viewmodel.InstallViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun InstallScreen(
    config: Screen.Install
) {
    val installViewModel = kodeinViewModel<InstallViewModel>(dispatcher = Dispatchers.IO)

    LaunchedEffect(config) {
        when (config) {
            is Screen.Install.Shrink.Local -> {
                installViewModel.shrinkInstallLocal(
                    diskId = config.diskId,
                    partitionId = config.partitionId,
                    isoPath = config.filePath
                )
            }
            is Screen.Install.Shrink.Remote -> {
                installViewModel.shrinkInstallRemote(
                    diskId = config.diskId,
                    partitionId = config.partitionId,
                    distroId = config.distroId
                )
            }
        }
    }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                modifier = Modifier.fillMaxWidth(),
                title = { WindowTitle("Installing") }
            )
        }
    ) {
        Text("Installing please wait, check the logs for progress")
    }
}