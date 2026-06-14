package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import dev.datlag.isekai.Symbols
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.viewmodel.SystemViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun SystemCheckScreen(onReady: () -> Unit) = DefaultScreen {
    val viewModel = kodeinViewModel<SystemViewModel>(dispatcher = Dispatchers.IO)
    val report by viewModel.systemReport.collectAsState()
    val error by viewModel.error.collectAsState()

    LaunchedEffect(viewModel) {
        println("Calling System Check...")
        viewModel.checkSystem()
    }

    LaunchedEffect(report) {
        if (report?.isReady == true) {
            onReady()
        }
    }

    if (error != null) {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = Symbols.NETWORK_ERROR,
            title = "Communication Error",
            description = error
        ) {
            Button(onClick = { viewModel.checkSystem() }) {
                Text("Retry")
            }
        }
    } else if (report == null) {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = Symbols.SEARCH,
            title = "Checking System...",
            description = "Verifying required dependencies."
        )
    } else if (!report!!.isReady) {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = Symbols.PACKAGES,
            title = "Missing Dependencies",
            description = "Required packages are missing on your system."
        ) {
            Button(onClick = { viewModel.checkSystem() }) {
                Text("Retry")
            }
        }
    } else {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = IconName("selection-mode-symbolic"),
            title = "System Check",
            description = "Required packages are available."
        )
    }
}