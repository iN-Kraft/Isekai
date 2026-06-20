package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.repository.InstallRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import org.kodein.di.DirectDI
import org.kodein.di.instance

class InstallViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: InstallRepository = instance()

    fun shrinkInstallLocal(diskId: String, partitionId: String, isoPath: String) {
        viewModelScope.launch {
            fold(
                block = { repository.shrinkInstallLocal(diskId, partitionId, isoPath) },
                catch = { e ->
                    e.printStackTrace()
                },
                recover = { err: IPCError ->
                    println(err)
                },
                transform = {
                    println("Install Finished!")
                }
            )
        }
    }

    fun uninstall(diskId: String) {
        viewModelScope.launch {
            fold(
                block = { repository.uninstall(diskId) },
                catch = { e ->
                    e.printStackTrace()
                },
                recover = { err: IPCError ->
                    println(err)
                },
                transform = { }
            )
        }
    }
}