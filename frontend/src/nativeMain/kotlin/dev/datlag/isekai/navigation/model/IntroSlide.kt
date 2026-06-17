package dev.datlag.isekai.navigation.model

import dev.datlag.isekai.translation.Introduction
import dev.datlag.kommons.gtk.compose.component.IconName

data class IntroSlide(
    val title: String,
    val description: String,
    val icon: IconName
) {

    companion object {
        val collection by lazy {
            listOf(
                IntroSlide(
                    title = Introduction.WELCOME_TITLE,
                    description = Introduction.WELCOME_TEXT,
                    icon = IconName("emblem-system-symbolic")
                ),
                IntroSlide(
                    title = Introduction.HARDWARE_TITLE,
                    description = Introduction.HARDWARE_TEXT,
                    icon = IconName("drive-harddisk-symbolic")
                ),
                IntroSlide(
                    title = Introduction.DATA_TITLE,
                    description = Introduction.DATA_TEXT,
                    icon = IconName("security-high-symbolic")
                ),
                IntroSlide(
                    title = Introduction.START_TITLE,
                    description = Introduction.START_TEXT,
                    icon = IconName("system-run-symbolic")
                )
            )
        }
    }
}
