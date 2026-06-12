package dev.datlag.isekai.module

import dev.datlag.kommons.gtk.glib.GLib
import kotlinx.cinterop.allocArray
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.toKString
import kotlinx.cinterop.toLong
import platform.windows.GetModuleFileNameW
import platform.windows.MAX_PATH
import platform.windows.SW_HIDE
import platform.windows.SW_SHOW
import platform.windows.ShellExecuteW
import platform.windows.WCHARVar

class DaemonLauncher(val debug: Boolean) : ExecutableAware {

    private val daemonPath: String
        get() {
            return executablePath?.let { "$it\\isekai-daemon.exe" } ?: "isekai-daemon.exe"
        }

    fun startBackend(): Boolean {
        return memScoped {
            val result = ShellExecuteW(
                hwnd = null,
                lpOperation = null,
                lpFile = daemonPath,
                lpParameters = null,
                lpDirectory = null,
                nShowCmd = if (debug) SW_SHOW else SW_HIDE
            )

            val instanceHandle = result.toLong()
            instanceHandle > 32
        }
    }

}