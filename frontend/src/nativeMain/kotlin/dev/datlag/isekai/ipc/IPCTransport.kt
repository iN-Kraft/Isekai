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
import platform.windows.CreateFileW
import platform.windows.DWORDVar
import platform.windows.FILE_FLAG_WRITE_THROUGH
import platform.windows.GENERIC_READ
import platform.windows.GENERIC_WRITE
import platform.windows.GetLastError
import platform.windows.HANDLE
import platform.windows.INVALID_HANDLE_VALUE
import platform.windows.OPEN_EXISTING
import platform.windows.ReadFile
import platform.windows.WriteFile

class IPCTransport(
    private val pipeName: String = "\\\\.\\pipe\\isekai_daemon"
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

    context(_: Raise<IPCError>)
    suspend fun connect() {
        val currentState = _connectionState.value
        if(currentState is ConnectionState.Connected || currentState is ConnectionState.Connecting) {
            return
        }

        _connectionState.update { ConnectionState.Connecting }

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
            val errCode = GetLastError()
            val error = IPCError.ConnectionFailed("Failed to open Named Pipe at $pipeName", errCode)
            _connectionState.update { ConnectionState.Error(error) }
            raise(error)
        }

        pipeHandle = handle as HANDLE
        _connectionState.update { ConnectionState.Connected }

        readJob = transportScope.launch { readLoop(handle) }
    }

    private suspend fun readLoop(handle: HANDLE) {
        var terminationError: IPCError? = null

        try {
            memScoped {
                val bytesRead = alloc<DWORDVar>()
                val headerBuffer = ByteArray(4)

                while (currentCoroutineContext().isActive) {
                    val headerSuccess = headerBuffer.usePinned { pinnedHeader ->
                        ReadFile(
                            hFile = handle,
                            lpBuffer = pinnedHeader.addressOf(0),
                            nNumberOfBytesToRead = 4u,
                            lpNumberOfBytesRead = bytesRead.ptr,
                            lpOverlapped = null
                        )
                    }

                    if (headerSuccess == 0 || bytesRead.value == 0u) {
                        terminationError = IPCError.Disconnected("Pipe stream ended or handle closed.")
                        break
                    }

                    val payloadLength = ((headerBuffer[0].toInt() and 0xFF) shl 24) or
                            ((headerBuffer[1].toInt() and 0xFF) shl 16) or
                            ((headerBuffer[2].toInt() and 0xFF) shl 8) or
                            (headerBuffer[3].toInt() and 0xFF)

                    if (payloadLength <= 0) {
                        continue
                    }

                    val payloadBuffer = ByteArray(payloadLength)
                    val payloadSuccess = payloadBuffer.usePinned { pinnedPayload ->
                        ReadFile(
                            hFile = handle,
                            lpBuffer = pinnedPayload.addressOf(0),
                            nNumberOfBytesToRead = payloadLength.toUInt(),
                            lpNumberOfBytesRead = bytesRead.ptr,
                            lpOverlapped = null
                        )
                    }

                    if (payloadSuccess == 0 || bytesRead.value == 0u) {
                        terminationError = IPCError.Disconnected("Pipe stream fragmented.")
                        break
                    }

                    handleIncomingFrame(payloadBuffer.decodeToString())
                }
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
                is OutgoingMessage.Event -> _events.emit(msg)
                is OutgoingMessage.Response -> {
                    pendingRequestsMutex.withLock {
                        pendingRequests.remove(msg.id)
                    }?.complete(Either.Right(msg))
                }
            }
        } catch (e: Throwable) {
            println("Failed to decode IPC frame: $jsonLine. Cause: ${e.message}")
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

            memScoped {
                val bytesWritten = alloc<DWORDVar>()

                totalBuffer.usePinned { pinned ->
                    val success = WriteFile(
                        hFile = handle,
                        lpBuffer = pinned.addressOf(0),
                        nNumberOfBytesToWrite = totalBuffer.size.toUInt(),
                        lpNumberOfBytesWritten = bytesWritten.ptr,
                        lpOverlapped = null
                    )

                    if (success == 0) {
                        throw IllegalStateException("Write failed. Win32 Code: ${GetLastError()}")
                    }
                }
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