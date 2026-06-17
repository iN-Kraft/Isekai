package dev.datlag.isekai.translation

object Introduction : TranslationScreen {

    override val screenIdentifier: String = "intro"

    val SKIP by lazy { tr("skip", "Skip") }
    val NEXT by lazy { tr("next", "Next") }
    val START by lazy { tr("start", "Start") }

    val WELCOME_TITLE by lazy { tr("welcome_title", "Welcome to a New World") }
    val WELCOME_TEXT by lazy { tr("welcome_text", "Experience Linux without the headache. No USB sticks to flash, no confusing BIOS settings to tweak and no complicated manuals.") }

    val HARDWARE_TITLE by lazy { tr("hardware_title", "No Hardware Required") }
    val HARDWARE_TEXT by lazy { tr("hardware_text", "Project Isekai securely handles everything right from Windows. It carefully makes room on your hard drive to create a cozy, isolated space for your new system.") }

    val DATA_TITLE by lazy { tr("data_title", "Your Data is Safe") }
    val DATA_TEXT by lazy { tr("data_text", "Windows isn't going anywhere. We set up a friendly boot menu so you can always choose between Windows and Linux every time you turn on your PC.") }

    val START_TITLE by lazy { tr("start_title", "Let's Get Started") }
    val START_TEXT by lazy { tr("start_text", "To safely prepare your drive, Project Isekai needs permission to adjust your storage. When you click Start, Windows will ask for Administrator access.") }
}