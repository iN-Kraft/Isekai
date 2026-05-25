package dev.datlag.isekai.ipc

import kotlinx.coroutines.CancellationException
import io.ktor.network.selector.SelectorManager
import io.ktor.network.sockets.*
import io.ktor.util.collections.ConcurrentMap
import io.ktor.utils.io.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.Json

sealed class ConnectionState {
    data object Disconnected : ConnectionState()
    data object Connecting : ConnectionState()
    data object Connected : ConnectionState()
    data class Error(val message: String, val exception: Throwable? = null) : ConnectionState()
}

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

    private val _connectionState = MutableStateFlow<ConnectionState>(ConnectionState.Disconnected)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _events = MutableSharedFlow<OutgoingMessage.Event>(extraBufferCapacity = 64)
    val events = _events.asSharedFlow()

    private val pendingRequests = ConcurrentMap<String, CompletableDeferred<OutgoingMessage.Response>>()

    /**
     * Connects to the backend and starts the read loop.
     * Emits state changes to [connectionState] without throwing exceptions on failure.
     */
    suspend fun connect() = withContext(Dispatchers.IO) {
        if (_connectionState.value is ConnectionState.Connected || _connectionState.value is ConnectionState.Connecting) {
            return@withContext
        }

        _connectionState.value = ConnectionState.Connecting
        
        try {
            val s = aSocket(selectorManager).tcp().connect(host, port)
            socket = s
            sendChannel = s.openWriteChannel(autoFlush = true)
            
            val receiveChannel = s.openReadChannel()
            
            _connectionState.value = ConnectionState.Connected
            
            readJob = CoroutineScope(Dispatchers.IO).launch {
                readLoop(receiveChannel)
            }
        } catch (e: Exception) {
            _connectionState.value = ConnectionState.Error("Failed to connect to backend at $host:$port", e)
            cleanup(IpcConnectionException("Connection failed", e))
        }
    }

    private suspend fun readLoop(channel: ByteReadChannel) {
        var error: Exception? = null
        try {
            while (currentCoroutineContext().isActive && !channel.isClosedForRead) {
                val line = channel.readLine() ?: break // Break on EOF
                try {
                    val msg = json.decodeFromString<OutgoingMessage>(line)
                    when (msg) {
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
            // Handle unexpected socket drops or errors (ignore routine cancellations)
            if (e !is CancellationException) {
                error = e
            }
        } finally {
            if (error != null) {
                _connectionState.value = ConnectionState.Error("Connection lost", error)
            } else if (_connectionState.value !is ConnectionState.Disconnected) {
                _connectionState.value = ConnectionState.Disconnected
            }
            cleanup(IpcConnectionException("Connection closed", error))
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

    /**
     * Cleanly closes the socket and resets the state to Disconnected.
     */
    fun disconnect() {
        readJob?.cancel()
        cleanup()
        _connectionState.value = ConnectionState.Disconnected
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
        disconnect()
        selectorManager.close()
    }
}
