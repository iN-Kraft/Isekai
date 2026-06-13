package dev.datlag.isekai

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.module.AppModule
import dev.datlag.isekai.module.DaemonLauncher
import dev.datlag.isekai.module.Translator
import dev.datlag.isekai.module.tr
import dev.datlag.isekai.navigation.ConnectionScreen
import dev.datlag.isekai.navigation.HomeScreen
import dev.datlag.isekai.navigation.IntroductionScreen
import dev.datlag.isekai.navigation.NavBackStack
import dev.datlag.isekai.navigation.NavHost
import dev.datlag.isekai.navigation.Screen
import dev.datlag.isekai.navigation.SystemCheckScreen
import dev.datlag.kommons.adwaita.compose.adwaitaApplication
import dev.datlag.kommons.adwaita.compose.component.AboutDialog
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.License
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.component.Image
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
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

    val appName = tr("app_name", "Project Isekai")
    GLib.setApplicationName(appName)
    GLib.setPrgname("Project Isekai") // Do not localize

    adwaitaApplication(
        applicationId = "dev.datlag.isekai",
        title = appName,
        args = args.asIterable()
    ) {
        CompositionLocalProvider(LocalDI provides di) {
            val backStack = remember { NavBackStack<Screen>(Screen.Introduction) }

            Scaffold(
                modifier = Modifier.fillMaxSize(),
                topBar = {
                    TopAppBar(
                        modifier = Modifier.fillMaxWidth(),
                        title = { WindowTitle(appName) },
                        actions = {
                            var aboutDialogVisible by remember { mutableStateOf(false) }

                            AboutDialog(
                                visible = aboutDialogVisible,
                                applicationName = appName,
                                applicationIcon = "dev.datlag.Isekai",
                                version = "1.0.0",
                                developerName = "iNKraft",
                                developers = listOf("Jeff Retz https://github.com/DatL4g"),
                                website = "https://datlag.dev",
                                supportUrl = "https://github.com/iN-Kraft/Isekai",
                                issueUrl = "https://github.com/iN-Kraft/Isekai/issues",
                                licenseType = License.GPL_3_0,
                                onClosed = { aboutDialogVisible = false }
                            )

                            Button(
                                modifier = Modifier.css("flat", "circular"),
                                onClick = {
                                    aboutDialogVisible = !aboutDialogVisible
                                }
                            ) {
                                Image(iconName = IconName("help-about-symbolic"))
                            }
                        }
                    )
                }
            ) {
                NavHost(backStack = backStack) { currentScreen ->
                    when (currentScreen) {
                        is Screen.Introduction -> {
                            IntroductionScreen(onNavigateNext = { nextScreen -> backStack.replaceAll(nextScreen) })
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
}