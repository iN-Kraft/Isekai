package dev.datlag.isekai.translation

import dev.datlag.isekai.ipc.IPCError

object Connection : TranslationScreen {

    override val screenIdentifier: String = "connection"

    val RETRY by lazy { tr("retry", "Retry") }

    val CONNECTING_TITLE by lazy { tr("connecting_title", "Connecting") }
    val CONNECTING_TEXT by lazy { tr("connecting_text", "Establishing a secure connection to the background service.") }

    val CONNECTED_TITLE by lazy { tr("connected_title", "Connected") }
    val CONNECTED_TEXT by lazy { tr("connected_text", "Connection established successfully.") }

    val DISCONNECTED_TITLE by lazy { tr("disconnected_title", "Disconnected") }
    val DISCONNECTED_TEXT by lazy { tr("disconnected_text", "The connection to the background service was lost.") }

    val ERROR_TITLE by lazy { tr("error_title", "Connection Error") }
    private val ERROR_TEXT by lazy { tr("error_text", "An error occurred: ") }

    fun errorText(error: IPCError): String {
        return ERROR_TEXT + error.toString()
    }
}