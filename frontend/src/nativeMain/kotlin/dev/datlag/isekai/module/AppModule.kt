package dev.datlag.isekai.module

import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.isekai.repository.DiskManagerRepository
import dev.datlag.isekai.viewmodel.AppViewModel
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
        bindSingleton { DiskManagerRepository(instance()) }
        bindFactory<CoroutineScope, AppViewModel> { scope ->
            AppViewModel(instance(), instance(), scope)
        }
    }

}