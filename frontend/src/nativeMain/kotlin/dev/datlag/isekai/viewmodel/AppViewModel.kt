package dev.datlag.isekai.viewmodel

import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.ipc.IpcTransport
import dev.datlag.isekai.ipc.ValidationReport
import dev.datlag.isekai.navigation.Screen
import dev.datlag.isekai.repository.DiskManagerRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class AppViewModel(
    val transport: IpcTransport,
    private val repository: DiskManagerRepository,
    private val scope: CoroutineScope
) {

    private val _currentScreen = MutableStateFlow<Screen>(Screen.Introduction)
    val currentScreen = _currentScreen.asStateFlow()

    private val _systemReport = MutableStateFlow<ValidationReport?>(null)
    val systemReport = _systemReport.asStateFlow()

    init {
        scope.launch {
            transport.connectionState.collect { state ->
                when (state) {
                    is ConnectionState.Disconnected, is ConnectionState.Error -> {
                        if (_currentScreen.value != Screen.Introduction) {
                            _currentScreen.update { Screen.Connection }
                        }
                    }

                    is ConnectionState.Connected -> {
                        if (_currentScreen.value == Screen.Introduction || _currentScreen.value == Screen.Connection) {
                            runSystemCheck()
                        }
                    }

                    else -> { }
                }
            }
        }
    }

    fun finishIntroduction() {
        _currentScreen.update { Screen.Connection }
        scope.launch { transport.connect() }
    }

    fun retryConnection() {
        scope.launch { transport.connect() }
    }

    fun retrySystemCheck() {
        runSystemCheck()
    }

    private fun runSystemCheck() {
        _currentScreen.update { Screen.SystemCheck }
        scope.launch {
            val result = repository.checkSystem()
            result.onSuccess { report ->
                _systemReport.update { report }
                if (report.isReady) {
                    _currentScreen.update { Screen.Home }
                }
            }
        }
    }
}