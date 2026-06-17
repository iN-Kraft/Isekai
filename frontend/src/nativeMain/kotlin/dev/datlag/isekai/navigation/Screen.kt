package dev.datlag.isekai.navigation

import kotlinx.serialization.Serializable

@Serializable
sealed interface Screen : NavKey {

    @Serializable
    data object Introduction : Screen

    @Serializable
    data object Connection : Screen

    @Serializable
    data object DistroSelection : Screen

    @Serializable
    sealed interface BlueprintScreen : Screen {

        @Serializable
        data class LocalFile(
            val filePath: String
        ) : BlueprintScreen

        @Serializable
        data class Download(
            val name: String,
            val edition: String?
        ) : BlueprintScreen
    }
}