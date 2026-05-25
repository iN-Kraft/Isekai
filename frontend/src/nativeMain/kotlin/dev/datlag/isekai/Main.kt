package dev.datlag.isekai

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.module.AppModule
import dev.datlag.isekai.navigation.HomeScreen
import dev.datlag.isekai.navigation.IntroductionScreen
import dev.datlag.isekai.navigation.Screen
import dev.datlag.isekai.navigation.SystemCheckScreen
import dev.datlag.isekai.viewmodel.AppViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.compose.adwaitaApplication
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import org.kodein.di.DI
import org.kodein.di.compose.LocalDI

fun main(args: Array<String>) = adwaitaApplication(
    applicationId = "dev.datlag.isekai",
    title = "Isekai",
    args = args.asIterable()
) {
    val di = remember { DI {
        import(AppModule.di)
    } }

    CompositionLocalProvider(LocalDI provides di) {
        val appViewModel = kodeinViewModel<AppViewModel>()

        Scaffold(
            modifier = Modifier.fillMaxSize(),
            topBar = {
                TopAppBar(
                    modifier = Modifier.fillMaxWidth(),
                    title = { WindowTitle("Isekai") },
                )
            }
        ) {
            val currentScreen by appViewModel.currentScreen.collectAsState()
            val connectionState by appViewModel.transport.connectionState.collectAsState()
            val systemReport by appViewModel.systemReport.collectAsState()

            when (currentScreen) {
                is Screen.Introduction -> {
                    IntroductionScreen(onSkip = { appViewModel.finishIntroduction() })
                }
                is Screen.Connection -> {
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
                is Screen.SystemCheck -> {
                    SystemCheckScreen(
                        report = systemReport,
                        onRetry = { appViewModel.retrySystemCheck() }
                    )
                }
                is Screen.Home -> {
                    HomeScreen()
                }
            }
        }
    }
}