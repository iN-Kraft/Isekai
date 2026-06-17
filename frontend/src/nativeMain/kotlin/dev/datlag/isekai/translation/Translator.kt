package dev.datlag.isekai.translation

import dev.datlag.isekai.intl.native.libintl_bind_textdomain_codeset
import dev.datlag.isekai.intl.native.libintl_bindtextdomain
import dev.datlag.isekai.intl.native.libintl_textdomain
import dev.datlag.isekai.module.ExecutableAware
import dev.datlag.kommons.gtk.glib.GLib
import dev.datlag.kommons.locale.Locale
import platform.posix.LC_ALL
import platform.posix.putenv
import platform.posix.setlocale

object Translator : ExecutableAware {
    private const val DOMAIN = "isekai"

    fun initialize() {
        setlocale(LC_ALL, "")

        Locale()?.language?.ifBlank { null }?.let {
            putenv("LANG=$it")
        }

        val safeExePath = executablePath?.replace('\\', '/')
        val localeDir = safeExePath?.let { "$it/share/locale" } ?: "share/locale"

        val boundPath = libintl_bindtextdomain(DOMAIN, localeDir)

        libintl_bind_textdomain_codeset(DOMAIN, "UTF-8")
        libintl_textdomain(DOMAIN)
    }

    fun translate(msgId: String, default: String): String {
        val translated = GLib.dgettext(DOMAIN, msgId) ?: default

        if (translated == msgId) {
            return default
        }
        return translated
    }
}