package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import dev.datlag.isekai.Symbols
import dev.datlag.isekai.ipc.ValidationReport
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize

@Composable
fun SystemCheckScreen(report: ValidationReport?, onRetry: () -> Unit) {
    if (report == null) {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = Symbols.SEARCH,
            title = "Checking System...",
            description = "Verifying required dependencies."
        )
    } else if (!report.isReady) {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = Symbols.PACKAGES,
            title = "Missing Dependencies",
            description = "Required packages are missing on your system."
        )
    } else {
        StatusPage(
            modifier = Modifier.fillMaxSize(),
            icon = IconName("selection-mode-symbolic"),
            title = "System Check",
            description = "Required packages are available."
        )
    }
}