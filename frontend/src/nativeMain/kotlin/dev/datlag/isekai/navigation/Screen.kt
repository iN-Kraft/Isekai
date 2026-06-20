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
            val filePath: String,
            val fileSize: ULong
        ) : BlueprintScreen

        @Serializable
        data class Download(
            val name: String,
            val edition: String?
        ) : BlueprintScreen
    }

    @Serializable
    sealed interface Install : Screen {

        @Serializable
        sealed interface Shrink : Install {

            @Serializable
            data class Local(
                val diskId: String,
                val partitionId: String,
                val filePath: String,
            ) : Shrink
        }
    }

    @Serializable
    data object Uninstall : Screen

    @Serializable
    data object Home : Screen
}