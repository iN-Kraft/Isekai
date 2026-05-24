package dev.datlag.isekai.ipc

import io.ktor.network.selector.SelectorManager
import io.ktor.network.sockets.Socket
import io.ktor.network.sockets.aSocket
import io.ktor.network.sockets.openReadChannel
import io.ktor.network.sockets.openWriteChannel
import io.ktor.util.collections.ConcurrentMap
import io.ktor.utils.io.ByteReadChannel
import io.ktor.utils.io.ByteWriteChannel
import io.ktor.utils.io.cancel
import io.ktor.utils.io.readLine
import io.ktor.utils.io.writeStringUtf8
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json

class IpcClient(private val port: Int = 45454) : AutoCloseable {

    private val json = Json {
        ignoreUnknownKeys = true
        classDiscriminator = "type"
    }

    private val requestJson = Json {
        classDiscriminator = "method"
    }

    private val selectorManager = SelectorManager(Dispatchers.IO)
    private var socket: Socket? = null
    private var sendChannel: ByteWriteChannel? = null

    private val _events = MutableSharedFlow<OutgoingMessage.Event>()
    val events = _events.asSharedFlow()

    private val pendingRequests = ConcurrentMap<String, CompletableDeferred<OutgoingMessage.Response>>()

    suspend fun connect() {
        try {
            socket = aSocket(selectorManager).tcp().connect("127.0.0.1", port)
            sendChannel = socket?.openWriteChannel(autoFlush = true)

            val receiveChannel = socket?.openReadChannel()
            println("Connected to Rust backend on port $port")

            CoroutineScope(Dispatchers.IO).launch {
                receiveChannel?.let { readLoop(it) }
            }
        } catch (e: Exception) {
            println("Failed to connect to Rust backend")
            e.printStackTrace()
            throw e
        }
    }

    private suspend fun readLoop(channel: ByteReadChannel) {
        try {
            while (!channel.isClosedForRead) {
                val line = channel.readLine() ?: break

                try {
                    when (val msg = json.decodeFromString<OutgoingMessage>(line)) {
                        is OutgoingMessage.Event -> {
                            _events.emit(msg)
                        }
                        is OutgoingMessage.Response -> {
                            pendingRequests.remove(msg.id)?.complete(msg)
                        }
                    }
                } catch (e: Exception) {
                    println("Failed to parse incoming JSON: $line")
                    e.printStackTrace()
                }
            }
        } catch (e: Exception) {
            println("Socket read loop terminated.")
        }
    }

    suspend fun <T : IpcRequest> sendCommand(request: T): OutgoingMessage.Response {
        val channel = sendChannel ?: throw IllegalStateException("Socket not connected.")
        val deferred = CompletableDeferred<OutgoingMessage.Response>()
        pendingRequests[request.id] = deferred

        val jsonString = requestJson.encodeToString(IpcRequest.serializer(), request)
        channel.writeStringUtf8("$jsonString\n")

        return deferred.await()
    }

    override fun close() {
        socket?.close()
        socket = null
        sendChannel = null
    }
}