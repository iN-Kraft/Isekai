package dev.datlag.isekai.navigation.component

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import dev.datlag.isekai.translation.TranslationScreen
import dev.datlag.isekai.translation.Translator
import dev.datlag.kommons.adwaita.compose.component.AboutDialog
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.SnackbarHostState
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.adwaita.compose.component.rememberSnackbarHostState
import dev.datlag.kommons.gtk.License
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.component.Image
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth

val LocalAppName = staticCompositionLocalOf<String> { Translator.translate("app_name", "Project Isekai") }

@Composable
fun <T : TranslationScreen> DefaultScreen(
    translation: T,
    showBackButton: Boolean = false,
    title: @Composable T.() -> Unit = { WindowTitle(title = LocalAppName.current) },
    content: @Composable T.(SnackbarHostState) -> Unit
) = with(translation) {
    val snackbarState = rememberSnackbarHostState()

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                modifier = Modifier.fillMaxWidth(),
                title = { title() },
                showBackButton = showBackButton,
                actions = {
                    var aboutDialogVisible by remember { mutableStateOf(false) }

                    AboutDialog(
                        visible = aboutDialogVisible,
                        applicationName = LocalAppName.current,
                        applicationIcon = "dev.datlag.Isekai",
                        version = "1.0.0",
                        developerName = "iNKraft",
                        developers = listOf("Jeff Retz https://github.com/DatL4g"),
                        website = "https://datlag.dev",
                        supportUrl = "https://github.com/iN-Kraft/Isekai",
                        issueUrl = "https://github.com/iN-Kraft/Isekai/issues",
                        licenseType = License.GPL_3_0,
                        copyright = "Copyright © Jeff Retz / iNKraft",
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
        },
        snackbarHostState = snackbarState,
        content = { content(snackbarState) }
    )
}