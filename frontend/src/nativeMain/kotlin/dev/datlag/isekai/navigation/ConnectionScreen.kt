package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import dev.datlag.isekai.Symbols
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.translation.Connection
import dev.datlag.isekai.viewmodel.ConnectionViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.compose.component.ButtonContent
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.gtk.compose.component.Button
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
) = DefaultScreen(Connection) {
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
            is ConnectionState.Connecting -> CONNECTING_TITLE
            is ConnectionState.Connected -> CONNECTED_TITLE
            is ConnectionState.Disconnected -> DISCONNECTED_TITLE
            is ConnectionState.Error -> ERROR_TITLE
        },
        description = when (val current = connectionState) {
            is ConnectionState.Connecting -> CONNECTING_TEXT
            is ConnectionState.Connected -> CONNECTED_TEXT
            is ConnectionState.Disconnected -> DISCONNECTED_TEXT
            is ConnectionState.Error -> errorText(current.error)
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
                ButtonContent(label = RETRY, iconName = "object-rotate-left-symbolic")
            }
        }
    }
}
