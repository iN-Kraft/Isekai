package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.translation.DistroSelection
import dev.datlag.kommons.adwaita.compose.component.ButtonRow
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css

@Composable
fun HomeScreen(
    onInstall: () -> Unit,
    onUninstall: () -> Unit
) = DefaultScreen(DistroSelection) {
    PreferencesPage {
        PreferencesGroup {
            ButtonRow(
                modifier = Modifier.css("suggested-action"),
                title = "Install",
                startIconName = "system-software-install-symbolic",
                onActivated = { onInstall() }
            )
            ButtonRow(
                title = "Uninstall",
                startIconName = "system-software-install-symbolic",
                onActivated = { onUninstall() }
            )
        }
    }
}