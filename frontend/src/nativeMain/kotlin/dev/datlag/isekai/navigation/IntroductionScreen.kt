package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.viewmodel.ConnectionViewModel
import dev.datlag.isekai.viewmodel.SystemViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.padding
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun IntroductionScreen(
    onNavigateNext: (Screen) -> Unit
) {
    val connectionViewModel = kodeinViewModel<ConnectionViewModel>(dispatcher = Dispatchers.IO)
    val systemViewModel = kodeinViewModel<SystemViewModel>(dispatcher = Dispatchers.IO)
    
    val connectionState by connectionViewModel.connectionState.collectAsState()
    val report by systemViewModel.systemReport.collectAsState()

    LaunchedEffect(connectionViewModel) {
        connectionViewModel.connect()
    }

    Column(
        modifier = Modifier.fillMaxSize(),
    ) {
        Text("Welcome to Isekai")
        Text("Migrate to a new Linux Distribution!")
        Button(onClick = {
            when (connectionState) {
                is ConnectionState.Connected if report?.isReady == true -> {
                    onNavigateNext(Screen.Home)
                }
                is ConnectionState.Connected -> {
                    onNavigateNext(Screen.SystemCheck)
                }
                else -> {
                    onNavigateNext(Screen.Connection)
                }
            }
        }, modifier = Modifier.padding(top = 16)) {
            Text("Skip & Start")
        }
    }
}