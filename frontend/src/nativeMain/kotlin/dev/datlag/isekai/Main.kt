package dev.datlag.isekai

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
    println("Isekai Frontend starting...")

    val client = IpcTransport()
    client.connect()

    delay(1000)

    val repo = DiskManagerRepository(client)
    val systemResult = repo.checkSystem()

    systemResult.onSuccess {
        println(it)
    }.onFailure {
        println(it)
    }

    val diskResult = repo.getDisks()
    diskResult.onSuccess {
        println(it)
    }.onFailure {
        println(it)
    }

    return@runBlocking
}