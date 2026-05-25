package dev.datlag.isekai.viewmodel

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.cancel
import org.kodein.di.DI
import org.kodein.di.DIAware
import org.kodein.di.DirectDI
import org.kodein.di.DirectDIAware
import org.kodein.di.compose.localDI
import org.kodein.di.factory
import kotlin.coroutines.EmptyCoroutineContext

abstract class KodeinViewModel(
    override val directDI: DirectDI,
    protected val viewModelScope: CoroutineScope
) : DirectDIAware {
    open fun onCleared() {
        viewModelScope.cancel()
    }
}

@Composable
inline fun <reified VM : KodeinViewModel> kodeinViewModel(
    di: DI = localDI(),
    dispatcher: CoroutineDispatcher = Dispatchers.Unconfined
): VM {
    val scope = rememberCoroutineScope(getContext = {
        EmptyCoroutineContext + dispatcher
    })
    val vm = remember(di, scope) { 
        val factory by di.factory<CoroutineScope, VM>()
        factory(scope)
    }

    DisposableEffect(vm) {
        onDispose {
            vm.onCleared()
        }
    }

    return vm
}
