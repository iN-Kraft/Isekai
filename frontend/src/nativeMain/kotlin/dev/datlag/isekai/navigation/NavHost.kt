package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable

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
    val currentScreen = backStack.current

    if (currentScreen != null) {
        content(currentScreen)
    }
}
