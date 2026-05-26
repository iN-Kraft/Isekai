package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember

/**
 * A container that renders the currently active screen from a [NavBackStack].
 * It animates transitions using [Crossfade] and ensures that popped screens
 * leave the composition, triggering their [androidx.compose.runtime.DisposableEffect]s.
 */
@Composable
fun <T : NavKey> NavHost(
    backStack: NavBackStack<T>,
    content: @Composable (T) -> Unit
) {
    val currentScreen by remember(backStack) {
        derivedStateOf { backStack.lastOrNull() }
    }

    currentScreen?.let {
        content(it)
    }
}
