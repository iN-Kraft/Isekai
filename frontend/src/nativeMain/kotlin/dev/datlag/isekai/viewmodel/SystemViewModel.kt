package dev.datlag.isekai.viewmodel

import dev.datlag.isekai.repository.SystemRepository
import kotlinx.coroutines.CoroutineScope
import org.kodein.di.DirectDI
import org.kodein.di.instance

class SystemViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val repository: SystemRepository = instance()

    
}
