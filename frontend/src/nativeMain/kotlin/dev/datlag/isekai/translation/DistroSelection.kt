package dev.datlag.isekai.translation

object DistroSelection : TranslationScreen {

    override val screenIdentifier: String = "distro_select"

    val DESKTOP by lazy { tr("desktop", "Desktop") }
    val GAMING by lazy { tr("gaming", "Gaming") }
    val DOWNLOAD by lazy { tr("download", "Download") }
    val BROWSE by lazy { tr("browse", "Browse") }

    val UNSURE_TITLE by lazy { tr("unsure_title", "Not sure which to choose?") }
    val UNSURE_TEXT by lazy { tr("unsure_text", "Test them in your browser first.") }

    val LOCAL_TITLE by lazy { tr("local_title", "Use your own ISO file") }
    val LOCAL_TEXT by lazy { tr("local_text", "Install a distribution you've already downloaded.") }
}