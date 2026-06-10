package dev.datlag.isekai.module

import kotlinx.cinterop.allocArray
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.toKString
import kotlinx.cinterop.toLong
import platform.windows.GetModuleFileNameW
import platform.windows.MAX_PATH
import platform.windows.SW_HIDE
import platform.windows.ShellExecuteW
import platform.windows.WCHARVar

object DaemonLauncher {

    private val daemonPath: String
        get() {
            return memScoped {
                val bufferLength = MAX_PATH
                val buffer = allocArray<WCHARVar>(bufferLength)

                GetModuleFileNameW(null, buffer, bufferLength.toUInt())

                val frontendPath = buffer.toKString()
                val lastSlashIndex = frontendPath.lastIndexOf('\\')
                if (lastSlashIndex >= 0) {
                    val dir = frontendPath.substring(0, lastSlashIndex)
                    "$dir\\isekai-daemon.exe"
                } else {
                    "isekai-daemon.exe"
                }
            }
        }

    fun startBackend(): Boolean {
        return memScoped {
            val result = ShellExecuteW(
                hwnd = null,
                lpOperation = null,
                lpFile = daemonPath,
                lpParameters = null,
                lpDirectory = null,
                nShowCmd = SW_HIDE
            )

            val instanceHandle = result.toLong()
            instanceHandle > 32
        }
    }

}