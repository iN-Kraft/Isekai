package dev.datlag.isekai.ipc

import arrow.core.Either
import arrow.core.raise.Raise
import arrow.core.raise.context.bind
import arrow.core.raise.context.ensure
import arrow.core.raise.context.ensureNotNull
import arrow.core.raise.context.raise
import arrow.core.raise.either
import arrow.core.raise.ensure
import arrow.core.raise.ensureNotNull
import dev.datlag.isekai.module.DaemonLauncher
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.alloc
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.usePinned
import kotlinx.cinterop.value
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import platform.windows.CloseHandle
import platform.windows.CreateEventW
import platform.windows.CreateFileW
import platform.windows.DWORDVar
import platform.windows.FILE_FLAG_WRITE_THROUGH
import platform.windows.GENERIC_READ
import platform.windows.GENERIC_WRITE
import platform.windows.GetLastError
import platform.windows.HANDLE
import platform.windows.INFINITE
import platform.windows.INVALID_HANDLE_VALUE
import platform.windows.OPEN_EXISTING
import platform.windows.PeekNamedPipe
import platform.windows.ReadFile
import platform.windows.WaitForSingleObject
import platform.windows.WriteFile

class IPCTransport(
    private val pipeName: String = "\\\\.\\pipe\\isekai_daemon",
    private val daemonLauncher: DaemonLauncher
) : AutoCloseable {

    private val json = Json {
        ignoreUnknownKeys = true
        classDiscriminator = "type"
    }

    private val requestJson = Json {
        classDiscriminator = "method"
    }

    private var pipeHandle: HANDLE? = null
    private var readJob: Job? = null

    private val transportScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val _connectionState = MutableStateFlow<ConnectionState>(ConnectionState.Disconnected)
    val connectionState = _connectionState.asStateFlow()

    private val _events = MutableSharedFlow<OutgoingMessage.Event>(extraBufferCapacity = 64)
    val events = _events.asSharedFlow()

    private val pendingRequestsMutex = Mutex()
    private val pendingRequests = mutableMapOf<String, CompletableDeferred<Either<IPCError, OutgoingMessage.Response>>>()

    fun connect() {
        val currentState = _connectionState.value
        if(currentState is ConnectionState.Connected || currentState is ConnectionState.Connecting) {
            return
        }

        _connectionState.update { ConnectionState.Connecting }

        transportScope.launch {
            val hEvent = withContext(Dispatchers.IO) {
                CreateEventW(
                    lpEventAttributes = null,
                    bManualReset = 1,
                    bInitialState = 0,
                    lpName = "Local\\IsekaiDaemonReady"
                )
            }

            if (hEvent == null || hEvent == INVALID_HANDLE_VALUE) {
                _connectionState.update { ConnectionState.Error(IPCError.ConnectionFailed("Failed to create sync event.", GetLastError())) }
                return@launch
            }

            try {
                val launchSuccess = daemonLauncher.startBackend()
                if (!launchSuccess) {
                    _connectionState.update { ConnectionState.Error(IPCError.ConnectionFailed("Failed to start daemon process.")) }
                    return@launch
                }

                withContext(Dispatchers.IO) {
                    WaitForSingleObject(hEvent, INFINITE)
                }

                val handle = withContext(Dispatchers.IO) {
                    CreateFileW(
                        lpFileName = pipeName,
                        dwDesiredAccess = (GENERIC_READ or GENERIC_WRITE.toUInt()),
                        dwShareMode = 0u,
                        lpSecurityAttributes = null,
                        dwCreationDisposition = OPEN_EXISTING.toUInt(),
                        dwFlagsAndAttributes = FILE_FLAG_WRITE_THROUGH,
                        hTemplateFile = null
                    )
                }

                if (handle == INVALID_HANDLE_VALUE) {
                    _connectionState.update {
                        ConnectionState.Error(
                            IPCError.ConnectionFailed(
                                "Pipe was signaled but failed to open.",
                                GetLastError()
                            )
                        )
                    }
                    return@launch
                }

                pipeHandle = handle as HANDLE
                _connectionState.update { ConnectionState.Connected }

                readJob = launch { readLoop(handle) }
            } catch (e: Throwable) {
                _connectionState.update {
                    ConnectionState.Error(IPCError.ConnectionFailed("Crash during connection: ${e.message}", null))
                }
            } finally {
                CloseHandle(hEvent)
            }
        }
    }

    private suspend fun readExact(handle: HANDLE, buffer: ByteArray): Boolean {
        var totalRead = 0
        val bytesToRead = buffer.size

        memScoped {
            val bytesRead = alloc<DWORDVar>()

            while (totalRead < bytesToRead) {
                val success = buffer.usePinned { pinned ->
                    ReadFile(
                        hFile = handle,
                        lpBuffer = pinned.addressOf(totalRead),
                        nNumberOfBytesToRead = (bytesToRead - totalRead).toUInt(),
                        lpNumberOfBytesRead = bytesRead.ptr,
                        lpOverlapped = null
                    )
                }

                if (success == 0 || bytesRead.value == 0u) {
                    return false
                }

                totalRead += bytesRead.value.toInt()
            }
        }

        return true
    }

    private suspend fun writeExact(handle: HANDLE, buffer: ByteArray): Boolean {
        var totalWritten = 0
        val bytesToWrite = buffer.size

        memScoped {
            val bytesWritten = alloc<DWORDVar>()

            while (totalWritten < bytesToWrite) {
                val success = buffer.usePinned { pinned ->
                    WriteFile(
                        hFile = handle,
                        lpBuffer = pinned.addressOf(totalWritten),
                        nNumberOfBytesToWrite = (bytesToWrite - totalWritten).toUInt(),
                        lpNumberOfBytesWritten = bytesWritten.ptr,
                        lpOverlapped = null
                    )
                }

                if (success == 0 || bytesWritten.value == 0u) {
                    return false
                }

                totalWritten += bytesWritten.value.toInt()
            }
        }

        return true
    }

    private suspend fun waitForBytes(handle: HANDLE, requiredBytes: UInt): Boolean {
        while (currentCoroutineContext().isActive) {
            var bytesAvailable = 0u
            var peekSuccess = 0

            memScoped {
                val avail = alloc<DWORDVar>()
                peekSuccess = PeekNamedPipe(
                    hNamedPipe = handle,
                    lpBuffer = null,
                    nBufferSize = 0u,
                    lpBytesRead = null,
                    lpTotalBytesAvail = avail.ptr,
                    lpBytesLeftThisMessage = null
                )
                bytesAvailable = avail.value
            }

            if (peekSuccess == 0) {
                return false
            }

            if (bytesAvailable >= requiredBytes) {
                return true
            }

            delay(10)
        }
        return false
    }

    private suspend fun readLoop(handle: HANDLE) {
        var terminationError: IPCError? = null

        try {
            val headerBuffer = ByteArray(4)

            while (currentCoroutineContext().isActive) {
                if (!waitForBytes(handle, headerBuffer.size.toUInt())) {
                    terminationError = IPCError.Disconnected("Pipe stream ended or closed.")
                    break
                }

                if (!readExact(handle, headerBuffer)) {
                    terminationError = IPCError.Disconnected("Pipe stream ended or header read failed.")
                    break
                }

                val payloadLength = ((headerBuffer[0].toInt() and 0xFF) shl 24) or
                        ((headerBuffer[1].toInt() and 0xFF) shl 16) or
                        ((headerBuffer[2].toInt() and 0xFF) shl 8) or
                        (headerBuffer[3].toInt() and 0xFF)

                if (payloadLength <= 0) {
                    continue
                }

                if (!waitForBytes(handle, payloadLength.toUInt())) {
                    terminationError = IPCError.Disconnected("Pipe stream ended during payload.")
                }

                val payloadBuffer = ByteArray(payloadLength)
                if (!readExact(handle, payloadBuffer)) {
                    terminationError = IPCError.Disconnected("Pipe stream ended during payload read.")
                    break
                }

                handleIncomingFrame(payloadBuffer.decodeToString())
            }
        } finally {
            val finalState = terminationError?.let { ConnectionState.Error(it) } ?: ConnectionState.Disconnected
            _connectionState.update { finalState }
            cleanup(terminationError)
        }
    }

    private suspend fun handleIncomingFrame(jsonLine: String) {
        try {
            when (val msg = json.decodeFromString<OutgoingMessage>(jsonLine)) {
                is OutgoingMessage.Event -> {
                    println("[EVENT] $msg")
                    _events.emit(msg)
                }
                is OutgoingMessage.Response -> {
                    pendingRequestsMutex.withLock {
                        pendingRequests.remove(msg.id)
                    }?.complete(Either.Right(msg.also { println("[RESPONSE] $it") }))
                }
            }
        } catch (e: Throwable) {
            println("Failed to decode IPC frame: $jsonLine. Cause: ${e.message}")
            cleanup(IPCError.SerializationError("Failed to parse backend response: ${e.message}"))
        }
    }

    context(_: Raise<IPCError>)
    suspend fun <T : IpcRequest> send(request: T): OutgoingMessage.Response {
        val handle = ensureNotNull(pipeHandle) {
            IPCError.Disconnected("Not connected to backend.")
        }
        val deferred = CompletableDeferred<Either<IPCError, OutgoingMessage.Response>>()

        pendingRequestsMutex.withLock {
            pendingRequests[request.id] = deferred
        }

        val sendResult = Either.catch {
            val jsonString = requestJson.encodeToString(IpcRequest.serializer(), request)
            val payloadBytes = jsonString.encodeToByteArray()
            val payloadSize = payloadBytes.size

            val headerBytes = byteArrayOf(
                ((payloadSize ushr 24) and 0xFF).toByte(),
                ((payloadSize ushr 16) and 0xFF).toByte(),
                ((payloadSize ushr 8) and 0xFF).toByte(),
                (payloadSize and 0xFF).toByte()
            )

            val totalBuffer = headerBytes + payloadBytes

            println("[SEND] ${totalBuffer.size} Bytes")
            val success = writeExact(handle, totalBuffer)

            if (!success) {
                println("[SEND] Failed")
                throw IllegalStateException("Write failed. Win32 Code: ${GetLastError()}")
            }
        }.mapLeft {
            IPCError.ConnectionFailed("Failed to send request frame", null)
        }

        if (sendResult.isLeft()) {
            pendingRequestsMutex.withLock {
                pendingRequests.remove(request.id)
            }
            sendResult.bind()
        }

        return try {
            val response = deferred.await().bind()

            ensure(response.success) {
                IPCError.BackendError(response.error ?: "Unknown backend error")
            }

            response
        } catch (e: CancellationException) {
            println("[SEND] Cancelled")
            pendingRequestsMutex.withLock {
                pendingRequests.remove(request.id)
            }
            raise(IPCError.RequestCancelled(request.id))
        }
    }

    fun disconnect() {
        val handle = pipeHandle
        pipeHandle = null

        if (handle != null) {
            CloseHandle(handle)
        }

        readJob?.cancel()
        _connectionState.update { ConnectionState.Disconnected }
    }

    private suspend fun cleanup(error: IPCError?) {
        pendingRequestsMutex.withLock {
            pendingRequests.forEach { (id, deferred) ->
                val result = error?.let { Either.Left(it) } ?: Either.Left(IPCError.RequestCancelled(id))
                deferred.complete(result)
            }
            pendingRequests.clear()
        }
    }

    override fun close() {
        disconnect()
        transportScope.cancel()
    }
}