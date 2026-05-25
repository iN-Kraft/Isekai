package dev.datlag.isekai

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import dev.datlag.isekai.module.AppModule
import dev.datlag.isekai.navigation.ConnectionScreen
import dev.datlag.isekai.navigation.HomeScreen
import dev.datlag.isekai.navigation.IntroductionScreen
import dev.datlag.isekai.navigation.NavBackStack
import dev.datlag.isekai.navigation.NavHost
import dev.datlag.isekai.navigation.Screen
import dev.datlag.isekai.navigation.SystemCheckScreen
import dev.datlag.kommons.adwaita.compose.adwaitaApplication
import dev.datlag.kommons.adwaita.compose.component.Scaffold
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
        val backStack = remember { NavBackStack<Screen>(Screen.Introduction) }

        Scaffold(
            modifier = Modifier.fillMaxSize(),
            topBar = {
                TopAppBar(
                    modifier = Modifier.fillMaxWidth(),
                    title = { WindowTitle("Isekai") },
                )
            }
        ) {
            NavHost(backStack = backStack) { currentScreen ->
                when (currentScreen) {
                    is Screen.Introduction -> {
                        IntroductionScreen(onSkip = { backStack.replaceCurrent(Screen.Connection) })
                    }
                    is Screen.Connection -> {
                        ConnectionScreen(
                            onConnected = { backStack.replaceAll(Screen.SystemCheck) }
                        )
                    }
                    is Screen.SystemCheck -> {
                        SystemCheckScreen(
                            onReady = { backStack.replaceAll(Screen.Home) }
                        )
                    }
                    is Screen.Home -> {
                        HomeScreen()
                    }
                }
            }
        }
    }
}