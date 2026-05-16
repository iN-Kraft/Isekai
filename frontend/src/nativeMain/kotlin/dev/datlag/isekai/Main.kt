package dev.datlag.isekai

import dev.datlag.isekai.domain.IpcClient
import dev.datlag.isekai.domain.IsekaiCommand
import dev.datlag.isekai.domain.IsekaiEvent
import io.ktor.network.selector.SelectorManager
import io.ktor.network.sockets.aSocket
import io.ktor.network.sockets.openReadChannel
import io.ktor.network.sockets.openWriteChannel
import io.ktor.utils.io.readLine
import io.ktor.utils.io.writeStringUtf8
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    println("Isekai Frontend starting...")

    val client = IpcClient()

    launch {
        client.events.collect { event ->
            when (event) {
                is IsekaiEvent.DisksLoaded -> {
                    println("\nKotlin received ${event.disks.size} disks from Rust!")
                    event.disks.forEach { disk ->
                        println("\t -> [${disk.diskNum}] ${disk.name} (${disk.totalGb} GB) System: ${disk.isSystemDrive}")
                    }

                    client.disconnect()
                }
                is IsekaiEvent.Progress -> println("Progress: ${event.percent}% - ${event.step}")
                is IsekaiEvent.FatalError -> println("Fatal error: ${event.message}")
            }
        }
    }

    launch {
        client.connect(45454)
    }

    delay(1000)
    client.sendCommand(IsekaiCommand.GetDisks)
}