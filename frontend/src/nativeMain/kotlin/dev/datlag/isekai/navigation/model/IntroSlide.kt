package dev.datlag.isekai.navigation.model

import dev.datlag.isekai.module.tr
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
                    title = tr("intro_welcome_title", "Welcome to a New World"),
                    description = tr("intro_welcome_text", "Experience Linux without the headache. No USB sticks to flash, no confusing BIOS settings to tweak and no complicated manuals."),
                    icon = IconName("emblem-system-symbolic")
                ),
                IntroSlide(
                    title = tr("intro_hardware_title", "No Hardware Required"),
                    description = tr("intro_hardware_text", "Project Isekai securely handles everything right from Windows. It carefully makes room on your hard drive to create a cozy, isolated space for your new system."),
                    icon = IconName("drive-harddisk-symbolic")
                ),
                IntroSlide(
                    title = tr("intro_data_title", "Your Data is Safe"),
                    description = tr("intro_data_text", "Windows isn't going anywhere. We set up a friendly boot menu so you can always choose between Windows and Linux every time you turn on your PC."),
                    icon = IconName("security-high-symbolic")
                ),
                IntroSlide(
                    title = tr("intro_start_title", "Let's Get Started"),
                    description = tr("intro_start_text", "To safely prepare your drive, Project Isekai needs permission to adjust your storage. When you click Start, Windows will ask for Administrator access."),
                    icon = IconName("system-run-symbolic")
                )
            )
        }
    }
}
