package dev.datlag.isekai

import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.ipc.DiskManagerRepository
import dev.datlag.isekai.ipc.IpcRequest
import dev.datlag.isekai.ipc.IpcTransport
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

@OptIn(ExperimentalUuidApi::class)
fun main() = runBlocking {
    val client = IpcTransport()

    launch {
        client.connectionState.collect { state ->
            when (state) {
                is ConnectionState.Connecting -> println("Isekai Frontend connecting...")
                is ConnectionState.Connected -> {
                    val repo = DiskManagerRepository(client)

                    println(repo.checkSystem().getOrNull())
                }
                is ConnectionState.Error -> {
                    println(state.message)
                    state.exception?.printStackTrace()
                }
                is ConnectionState.Disconnected -> println("Isekai Frontend disconnected...")
            }
        }
    }

    client.connect()
}