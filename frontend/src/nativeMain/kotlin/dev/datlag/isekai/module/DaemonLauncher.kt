package dev.datlag.isekai.module

import kotlinx.cinterop.memScoped
import kotlinx.cinterop.toLong
import kotlinx.cinterop.wcstr
import platform.windows.SW_HIDE
import platform.windows.ShellExecuteW

object DaemonLauncher {

    fun startBackend(executablePath: String = "isekai_daemon.exe"): Boolean {
        return memScoped {
            val result = ShellExecuteW(
                hwnd = null,
                lpOperation = null,
                lpFile = executablePath,
                lpParameters = null,
                lpDirectory = null,
                nShowCmd = SW_HIDE
            )

            val instanceHandle = result.toLong()
            instanceHandle > 32
        }
    }

}