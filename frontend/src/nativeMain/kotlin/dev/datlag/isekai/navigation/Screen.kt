package dev.datlag.isekai.navigation

import kotlinx.serialization.Serializable

@Serializable
sealed interface Screen : NavKey {

    @Serializable
    data object Introduction : Screen

    @Serializable
    data object Connection : Screen

    @Serializable
    data object SystemCheck : Screen

    @Serializable
    data object OSSelection : Screen

    @Serializable
    data object Home : Screen
}