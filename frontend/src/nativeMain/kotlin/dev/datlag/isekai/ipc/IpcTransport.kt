package dev.datlag.isekai.ipc

import io.ktor.network.selector.SelectorManager
import io.ktor.network.sockets.*
import io.ktor.util.collections.ConcurrentMap
import io.ktor.utils.io.*
import kotlinx.coroutines.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.serialization.json.Json

/**
 * Custom exception for IPC communication errors.
 */
class IpcConnectionException(message: String, cause: Throwable? = null) : Exception(message, cause)

/**
 * Robust TCP engine for IPC communication with the Rust backend.
 * Handles socket lifecycle, JSON-Lines framing, and request/response matching.
 */
class IpcTransport(
    private val host: String = "127.0.0.1",
    private val port: Int = 45454
) : AutoCloseable {

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
    private var readJob: Job? = null

    private val _events = MutableSharedFlow<OutgoingMessage.Event>(extraBufferCapacity = 64)
    val events = _events.asSharedFlow()

    private val pendingRequests = ConcurrentMap<String, CompletableDeferred<OutgoingMessage.Response>>()

    /**
     * Connects to the backend and starts the read loop.
     */
    suspend fun connect() = withContext(Dispatchers.IO) {
        try {
            val s = aSocket(selectorManager).tcp().connect(host, port)
            socket = s
            sendChannel = s.openWriteChannel(autoFlush = true)
            
            val receiveChannel = s.openReadChannel()
            
            readJob = CoroutineScope(Dispatchers.IO).launch {
                readLoop(receiveChannel)
            }
        } catch (e: Exception) {
            throw IpcConnectionException("Failed to connect to backend at $host:$port", e)
        }
    }

    private suspend fun readLoop(channel: ByteReadChannel) {
        try {
            while (currentCoroutineContext().isActive && !channel.isClosedForRead) {
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
                    // Skip malformed lines or unknown messages
                }
            }
        } catch (e: Exception) {
            // Handle unexpected socket drops or errors
        } finally {
            cleanup(IpcConnectionException("Connection closed"))
        }
    }

    /**
     * Sends a request and waits for the matching response.
     */
    suspend fun <T : IpcRequest> send(request: T): OutgoingMessage.Response {
        val channel = sendChannel ?: throw IpcConnectionException("Not connected to backend")
        
        val deferred = CompletableDeferred<OutgoingMessage.Response>()
        pendingRequests[request.id] = deferred

        try {
            val jsonString = requestJson.encodeToString(IpcRequest.serializer(), request)
            channel.writeStringUtf8("$jsonString\n")
        } catch (e: Exception) {
            pendingRequests.remove(request.id)
            throw IpcConnectionException("Failed to send request", e)
        }

        return try {
            deferred.await()
        } catch (e: CancellationException) {
            pendingRequests.remove(request.id)
            throw e
        }
    }

    private fun cleanup(exception: Exception? = null) {
        socket?.close()
        socket = null
        sendChannel = null
        
        pendingRequests.forEach { (_, deferred) ->
            if (exception != null) {
                deferred.completeExceptionally(exception)
            } else {
                deferred.cancel()
            }
        }
        pendingRequests.clear()
    }

    override fun close() {
        readJob?.cancel()
        cleanup()
        selectorManager.close()
    }
}
