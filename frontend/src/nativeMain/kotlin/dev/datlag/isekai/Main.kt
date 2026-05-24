package dev.datlag.isekai

import dev.datlag.isekai.ipc.IpcClient
import dev.datlag.isekai.ipc.IpcRequest
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

@OptIn(ExperimentalUuidApi::class)
fun main() = runBlocking {
    println("Isekai Frontend starting...")

    val client = IpcClient()

    try {
        client.connect()

        val eventJob = launch {
            client.events.collect { event ->
                val percentStr = event.percent?.let { "[$it%]" } ?: ""
                println("[LIVE EVENT] ${event.eventType} $percentStr: ${event.message}")
            }
        }

        delay(1000)

        val checkId = Uuid.random().toString()
        val checkResponse = client.sendCommand(IpcRequest.CheckSystem(checkId))

        if (checkResponse.success) {
            println("Check Success!")
            println("Data: ${checkResponse.data}")
        } else {
            println("Check Failed: ${checkResponse.error}")
        }

        val disksId = Uuid.random().toString()
        val disksResponse = client.sendCommand(IpcRequest.GetDisks(disksId))

        if (disksResponse.success) {
            println("Disks Success!")
            println("Data: ${disksResponse.data}")
        } else {
            println("Disks Failed: ${disksResponse.error}")
        }

        delay(3000)
        eventJob.cancel()
        println("Client shutting down.")
    } catch (e: Exception) {
        println("CRITICAL FAILURE")
        e.printStackTrace()
    }
}