package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import dev.datlag.isekai.Symbols
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.module.tr
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.viewmodel.ConnectionViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.compose.component.ButtonContent
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.delay
import kotlin.time.Duration.Companion.seconds

@Composable
fun ConnectionScreen(
    onConnected: () -> Unit
) = DefaultScreen {
    val viewModel = kodeinViewModel<ConnectionViewModel>(dispatcher = Dispatchers.IO)
    val connectionState by viewModel.connectionState.collectAsState()

    LaunchedEffect(Unit) {
        viewModel.connect()
    }

    LaunchedEffect(connectionState) {
        if (connectionState is ConnectionState.Connected) {
            delay(2.seconds)
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
            is ConnectionState.Connecting -> tr("connection_connecting_title", "Connecting")
            is ConnectionState.Connected -> tr("connection_connected_title", "Connected")
            is ConnectionState.Disconnected -> tr("connection_disconnected_title", "Disconnected")
            is ConnectionState.Error -> tr("connection_error_title", "Connection Error")
        },
        description = when (val current = connectionState) {
            is ConnectionState.Connecting -> tr("connection_connecting_text", "Establishing a secure connection to the background service.")
            is ConnectionState.Connected -> tr("connection_connected_text", "Connection established successfully.")
            is ConnectionState.Disconnected -> tr("connection_disconnected_text", "The connection to the background service was lost.")
            is ConnectionState.Error -> tr("connection_error_text", "An error occurred: ") + current.error.toString()
        }
    ) {
        if (connectionState is ConnectionState.Error) {
            Button(
                modifier = Modifier.css("suggested-action", "pill"),
                onClick = {
                    viewModel.disconnect()
                    viewModel.connect()
                }
            ) {
                ButtonContent(label = tr("connection_retry", "Retry"), iconName = "object-rotate-left-symbolic")
            }
        }
    }
}
