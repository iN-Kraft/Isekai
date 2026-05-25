package dev.datlag.isekai

import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.kommons.adwaita.compose.adwaitaApplication
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth

fun main(args: Array<String>) = adwaitaApplication(
    applicationId = "dev.datlag.isekai",
    title = "Isekai",
    args = args.asIterable()
) {
    val ipcClient = remember { IpcTransport() }

    LaunchedEffect(ipcClient) {
        ipcClient.connect()
    }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                modifier = Modifier.fillMaxWidth(),
                title = { WindowTitle("Isekai") },
            )
        }
    ) {
        val connectionState by ipcClient.connectionState.collectAsState()

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
            }
        )
    }
}