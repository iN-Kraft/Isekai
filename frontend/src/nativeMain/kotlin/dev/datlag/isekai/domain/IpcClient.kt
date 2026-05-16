package dev.datlag.isekai.domain

import io.ktor.network.selector.SelectorManager
import io.ktor.network.sockets.Socket
import io.ktor.network.sockets.aSocket
import io.ktor.network.sockets.openReadChannel
import io.ktor.network.sockets.openWriteChannel
import io.ktor.utils.io.ByteWriteChannel
import io.ktor.utils.io.readLine
import io.ktor.utils.io.writeStringUtf8
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json

class IpcClient {

    private val selectorManager = SelectorManager(Dispatchers.IO)
    private var socket: Socket? = null
    private var sendChannel: ByteWriteChannel? = null

    private val _events = MutableSharedFlow<IsekaiEvent>()
    val events = _events.asSharedFlow()

    private val json = Json { ignoreUnknownKeys = true }

    suspend fun connect(port: Int) = withContext(Dispatchers.IO) {
        try {
            println("Kotlin: Connecting to Rust daemon on port $port...")
            socket = aSocket(selectorManager).tcp().connect("127.0.0.1", port)

            val receiveChannel = socket!!.openReadChannel()
            sendChannel = socket!!.openWriteChannel(autoFlush = true)

            println("Kotlin: Connected successfully! Starting listening loop...")

            while (isActive) {
                val line = receiveChannel.readLine() ?: break
                parseAndEmit(line)
            }
        } catch (e: Exception) {
            println("Kotlin: Socket disconnected or failed.")
            e.printStackTrace()
        } finally {
            disconnect()
        }
    }

    suspend fun sendCommand(command: IsekaiCommand) {
        val channel = sendChannel ?: return println("Kotlin: Cannot send, not connected.")

        try {
            val jsonString = json.encodeToString(command) + "\n"
            channel.writeStringUtf8(jsonString)
            println("Kotlin: Sent -> ${jsonString.trim()}")
        } catch (e: Exception) {
            println("Kotlin: Failed to send command")
            e.printStackTrace()
        }
    }

    private suspend fun parseAndEmit(jsonString: String) {
        try {
            val event = json.decodeFromString<IsekaiEvent>(jsonString)
            _events.emit(event)
        } catch (e: Exception) {
            println("Kotlin: Failed to parse JSON from Rust: $jsonString")
            e.printStackTrace()
        }
    }

    fun disconnect() {
        socket?.close()
        selectorManager.close()
    }

}