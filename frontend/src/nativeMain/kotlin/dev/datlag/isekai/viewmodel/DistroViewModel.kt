package dev.datlag.isekai.viewmodel

import arrow.core.raise.fold
import dev.datlag.isekai.common.withConfig
import dev.datlag.isekai.ipc.IPCError
import dev.datlag.isekai.navigation.model.DistroList
import dev.datlag.isekai.repository.DistroRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.kodein.di.DirectDI
import org.kodein.di.instance

class DistroViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: DistroRepository = instance()

    private val _desktopDistros = MutableStateFlow(DistroList.desktop)
    val desktopDistros = _desktopDistros.asStateFlow()

    private val _gamingDistros = MutableStateFlow(DistroList.gaming)
    val gamingDistros = _gamingDistros.asStateFlow()

    init {
        loadDistroConfig()
    }

    private fun loadDistroConfig() {
        viewModelScope.launch {
            fold(
                block = { repository.getDistroInfo() },
                catch = { e ->
                    e.printStackTrace()
                },
                recover = { err: IPCError ->
                    println(err)
                },
                transform = { config ->
                    _desktopDistros.update { it.withConfig(config) }
                    _gamingDistros.update { it.withConfig(config) }
                }
            )
        }
    }
}