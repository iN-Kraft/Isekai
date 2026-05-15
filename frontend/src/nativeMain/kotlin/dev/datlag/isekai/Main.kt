package dev.datlag.isekai

import io.ktor.network.selector.SelectorManager
import io.ktor.network.sockets.aSocket
import io.ktor.network.sockets.openReadChannel
import io.ktor.network.sockets.openWriteChannel
import io.ktor.utils.io.readLine
import io.ktor.utils.io.writeStringUtf8
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    println("Isekai Frontend starting...")

    val selectorManager = SelectorManager(Dispatchers.IO)

    try {
        println("Connecting to Rust daemon on port 45454...")
        val socket = aSocket(selectorManager).tcp().connect("127.0.0.1", 45454)
        val receiveChannel = socket.openReadChannel()
        val sendChannel = socket.openWriteChannel(autoFlush = true)
        val message = """{"command": "Hello from Project Isekai UI!"}\n"""
        println("Sending message: $message")
        sendChannel.writeStringUtf8(message)

        val response = receiveChannel.readLine()
        println("Response from Rust: $response")

        socket.close()
    } catch (e: Exception) {
        println("Failed to connect to Rust daemon. Is it running? Error: ${e.message}")
    } finally {
        selectorManager.close()
    }
}