package dev.datlag.isekai.module

import kotlinx.cinterop.allocArray
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.toKString
import platform.windows.GetModuleFileNameW
import platform.windows.MAX_PATH
import platform.windows.WCHARVar

interface ExecutableAware {

    val executablePath: String?
        get() {
            return memScoped {
                val bufferLength = MAX_PATH
                val buffer = allocArray<WCHARVar>(bufferLength)

                GetModuleFileNameW(null, buffer, bufferLength.toUInt())

                val frontendPath = buffer.toKString()
                val lastSlashIndex = frontendPath.lastIndexOf('\\')
                if (lastSlashIndex >= 0) {
                    frontendPath.substring(0, lastSlashIndex)
                } else {
                    null
                }
            }
        }

}