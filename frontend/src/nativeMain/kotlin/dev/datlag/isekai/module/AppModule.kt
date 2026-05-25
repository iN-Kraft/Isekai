package dev.datlag.isekai.module

import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.isekai.repository.DiskRepository
import dev.datlag.isekai.repository.SystemRepository
import dev.datlag.isekai.viewmodel.ConnectionViewModel
import dev.datlag.isekai.viewmodel.DiskViewModel
import dev.datlag.isekai.viewmodel.SystemViewModel
import kotlinx.coroutines.CoroutineScope
import org.kodein.di.DI
import org.kodein.di.bindFactory
import org.kodein.di.bindSingleton
import org.kodein.di.instance

object AppModule {

    private const val NAME = "AppModule"
    val di: DI.Module = DI.Module(NAME) {
        bindSingleton<IpcTransport> {
            IpcTransport()
        }
        
        bindSingleton { SystemRepository(instance()) }
        bindSingleton { DiskRepository(instance()) }

        bindFactory<CoroutineScope, ConnectionViewModel> { scope ->
            ConnectionViewModel(directDI = this, viewModelScope = scope)
        }

        bindFactory<CoroutineScope, SystemViewModel> { scope ->
            SystemViewModel(directDI = this, viewModelScope = scope)
        }

        bindFactory<CoroutineScope, DiskViewModel> { scope ->
            DiskViewModel(directDI = this, viewModelScope = scope)
        }
    }

}