package dev.datlag.isekai.viewmodel

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.CoroutineScope
import org.kodein.di.DI
import org.kodein.di.compose.localDI
import org.kodein.di.factory

@Composable
inline fun <reified VM : Any> kodeinViewModel(
    di: DI = localDI()
): VM {
    val vmFactory by di.factory<CoroutineScope, VM>()
    val scope = rememberCoroutineScope()
    val vm = remember(vmFactory, scope) { vmFactory(scope) }

    return vm
}