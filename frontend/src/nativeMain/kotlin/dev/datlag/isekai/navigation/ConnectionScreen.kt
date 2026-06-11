package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import dev.datlag.isekai.Symbols
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.viewmodel.ConnectionViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun ConnectionScreen(
    onConnected: () -> Unit
) {
    val viewModel = kodeinViewModel<ConnectionViewModel>(dispatcher = Dispatchers.IO)
    val connectionState by viewModel.connectionState.collectAsState()

    LaunchedEffect(Unit) {
        viewModel.connect()
    }

    LaunchedEffect(connectionState) {
        if (connectionState is ConnectionState.Connected) {
            onConnected()
        }
    }

    StatusPage(
        modifier = Modifier.fillMaxSize(),
        icon = when (connectionState) {
            is ConnectionState.Connecting -> Symbols.NETWORK_CONNECTING
            is ConnectionState.Connected -> Symbols.NETWORK_CONNECTED
            is ConnectionState.Disconnected -> Symbols.NETWORK_DISCONNECTED
            is ConnectionState.Error -> Symbols.NETWORK_ERROR
        },
        title = when (connectionState) {
            is ConnectionState.Connecting -> "Connecting"
            is ConnectionState.Connected -> "Connected"
            is ConnectionState.Disconnected -> "Disconnected"
            is ConnectionState.Error -> "Connection Error"
        },
        description = if (connectionState is ConnectionState.Error) {
            (connectionState as ConnectionState.Error).error.toString()
        } else {
            null
        }
    ) {
        if (connectionState is ConnectionState.Error) {
            Button(onClick = {
                viewModel.disconnect()
                viewModel.connect()
            }) {
                Text("Retry")
            }
        }
    }
}
