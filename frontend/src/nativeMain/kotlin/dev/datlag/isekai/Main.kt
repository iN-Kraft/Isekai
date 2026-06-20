package dev.datlag.isekai

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import dev.datlag.isekai.module.AppModule
import dev.datlag.isekai.module.DaemonLauncher
import dev.datlag.isekai.translation.Translator
import dev.datlag.isekai.navigation.BlueprintScreen
import dev.datlag.isekai.navigation.ConnectionScreen
import dev.datlag.isekai.navigation.IntroductionScreen
import dev.datlag.isekai.navigation.NavBackStack
import dev.datlag.isekai.navigation.NavHost
import dev.datlag.isekai.navigation.DistroSelectionScreen
import dev.datlag.isekai.navigation.HomeScreen
import dev.datlag.isekai.navigation.InstallScreen
import dev.datlag.isekai.navigation.Screen
import dev.datlag.isekai.navigation.UninstallScreen
import dev.datlag.isekai.navigation.component.LocalAppName
import dev.datlag.kommons.adwaita.compose.adwaitaApplication
import dev.datlag.kommons.gtk.glib.GLib
import org.kodein.di.DI
import org.kodein.di.bindSingleton
import org.kodein.di.compose.LocalDI

fun main(args: Array<String>) {
    val isSafeMode = args.contains("--safe-mode") || args.contains("--software-render")
    if (isSafeMode) {
        println("Safe mode detected: Forcing Cairo software rendering.")
        GLib.setenv("GSK_RENDERER", "cairo", true)
    }

    val di = DI {
        import(AppModule.di)

        bindSingleton<DaemonLauncher> {
            DaemonLauncher(debug = args.contains("--debug"))
        }
    }
    Translator.initialize()

    val appName = Translator.translate("app_name", "Project Isekai")
    GLib.setApplicationName(appName)
    GLib.setPrgname("Project Isekai") // Do not localize

    adwaitaApplication(
        applicationId = "dev.datlag.isekai",
        title = appName,
        args = args.asIterable()
    ) {
        CompositionLocalProvider(
            LocalDI provides di,
            LocalAppName provides appName
        ) {
            val backStack = remember { NavBackStack<Screen>(Screen.Introduction) }

            NavHost(backStack = backStack) { currentScreen ->
                when (currentScreen) {
                    is Screen.Introduction -> {
                        IntroductionScreen(onNavigateNext = { nextScreen -> backStack.replaceAll(nextScreen) })
                    }
                    is Screen.Connection -> {
                        ConnectionScreen(
                            onConnected = { backStack.replaceAll(Screen.Home) }
                        )
                    }
                    is Screen.Home -> {
                        HomeScreen(onInstall = { backStack.push(Screen.DistroSelection) }, onUninstall = { backStack.push(Screen.Uninstall) })
                    }
                    is Screen.DistroSelection -> {
                        DistroSelectionScreen(
                            onSelected = { screen -> backStack.push(screen) },
                        )
                    }
                    is Screen.BlueprintScreen -> {
                        BlueprintScreen(config = currentScreen, onBack = { backStack.pop() }, onInstall = { backStack.push(it) })
                    }
                    is Screen.Install -> {
                        InstallScreen(config = currentScreen)
                    }
                    is Screen.Uninstall -> {
                        UninstallScreen(onBack = { backStack.pop() })
                    }
                }
            }
        }
    }
}