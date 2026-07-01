package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ComposeNode
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import dev.datlag.isekai.viewmodel.InstallViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.SpinnerPaintable
import dev.datlag.kommons.adwaita.compose.component.ButtonContent
import dev.datlag.kommons.adwaita.compose.component.Clamp
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.adwaita.compose.component.StatusPageNode
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.WindowTitle
import dev.datlag.kommons.gtk.Justification
import dev.datlag.kommons.gtk.compose.GtkApplier
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.HorizontalDivider
import dev.datlag.kommons.gtk.compose.component.IconName
import dev.datlag.kommons.gtk.compose.component.LinearProgressIndicator
import dev.datlag.kommons.gtk.compose.component.Row
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import dev.datlag.kommons.gtk.glib.GLib
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import platform.posix.exit
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt
import dev.datlag.isekai.viewmodel.InstallViewModel.State as State

@Composable
fun InstallScreen(
    config: Screen.Install
) {
    val installViewModel = kodeinViewModel<InstallViewModel>(dispatcher = Dispatchers.IO)
    val state by installViewModel.state.collectAsState()

    LaunchedEffect(config) {
        when (config) {
            is Screen.Install.Shrink.Local -> {
                installViewModel.shrinkInstallLocal(
                    diskId = config.diskId,
                    partitionId = config.partitionId,
                    isoPath = config.filePath
                )
            }
            is Screen.Install.Shrink.Remote -> {
                installViewModel.shrinkInstallRemote(
                    diskId = config.diskId,
                    partitionId = config.partitionId,
                    distroId = config.distroId
                )
            }
        }
    }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                modifier = Modifier.fillMaxWidth(),
                navigationIcon = {
                    Button(
                        onClick = { installViewModel.cancelWorkflow() },
                        enabled = state is State.Running
                    ) {
                        ButtonContent(
                            label = "Cancel",
                            iconName = "media-playback-stop-symbolic"
                        )
                    }
                },
                title = { WindowTitle("Installing") }
            )
        }
    ) {
        when (val currentState = state) {
            is State.Idle -> {
                LoadingStatusPage(
                    modifier = Modifier.fillMaxSize(),
                    title = "Preparing..."
                )
            }
            is State.Success -> {
                StatusPage(
                    modifier = Modifier.fillMaxSize(),
                    title = "Installation Complete",
                    icon = IconName("selection-mode-symbolic")
                ) {
                    Clamp {
                        Button(
                            modifier = Modifier.css("pill", "suggested-action"),
                            onClick = { exit(0) },
                        ) {
                            ButtonContent(label = "Exit", iconName = "application-exit-symbolic")
                        }
                    }
                }
            }
            is State.Error -> {
                StatusPage(
                    modifier = Modifier.fillMaxSize(),
                    title = "Installation Error",
                    description = currentState.message,
                    icon = IconName("dialog-error-symbolic")
                )
            }
            is State.Running -> {
                when (currentState) {
                    is State.Running.Indeterminate -> {
                        LoadingStatusPage(
                            modifier = Modifier.fillMaxSize(),
                            title = currentState.title
                        )
                    }
                    is State.Running.Downloading -> {
                        if (currentState.isPaused) {
                            StatusPage(
                                modifier = Modifier.fillMaxSize(),
                                title = currentState.title,
                                icon = IconName("media-playback-pause-symbolic")
                            ) {
                                Clamp {
                                    Button(
                                        modifier = Modifier.css("pill", "suggested-action"),
                                        onClick = { installViewModel.togglePause() }
                                    ) {
                                        ButtonContent(label = "Resume", iconName = "media-playback-start-symbolic")
                                    }
                                }
                            }
                        } else {
                            val downloadedBytesText by remember(currentState.downloadedBytes) { derivedStateOf {
                                GLib.formatSizeForDisplay(currentState.downloadedBytes.toLong())
                            } }
                            val totalBytesText by remember(currentState.totalBytes) { derivedStateOf {
                                GLib.formatSizeForDisplay(currentState.totalBytes.toLong())
                            } }
                            val etaText by remember(currentState.etaSeconds) { derivedStateOf {
                                currentState.formatETA()
                            } }

                            LoadingStatusPage(
                                modifier = Modifier.fillMaxSize(),
                                title = currentState.title
                            ) {
                                Clamp {
                                    Column(modifier = Modifier.fillMaxWidth()) {
                                        val animatedProgress = animateFractionAsState(currentState.progress)

                                        LinearProgressIndicator(progress = animatedProgress)
                                        Row(
                                            modifier = Modifier.fillMaxWidth()
                                        ) {
                                            if (currentState.downloadedBytes <= 0uL || currentState.totalBytes <= 0uL) {
                                                Text(
                                                    text = "${(currentState.progress * 100F).roundToInt()}%",
                                                    textAlign = Justification.CENTER
                                                )
                                            } else {
                                                Text(
                                                    text = "$downloadedBytesText / $totalBytesText",
                                                    textAlign = Justification.CENTER
                                                )
                                            }
                                            HorizontalDivider(modifier = Modifier.css("spacer").weight(1F))
                                            Text(
                                                text = etaText.ifBlank { "00m 00s" },
                                                textAlign = Justification.CENTER
                                            )
                                        }

                                        if (!currentState.isPaused) {
                                            Button(
                                                modifier = Modifier.css("pill"),
                                                onClick = { installViewModel.togglePause() }
                                            ) {
                                                ButtonContent(label = "Pause", iconName = "media-playback-pause-symbolic")
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    is State.Running.Installing -> {
                        LoadingStatusPage(
                            modifier = Modifier.fillMaxSize(),
                            title = currentState.title
                        ) {
                            Clamp {
                                Column(modifier = Modifier.fillMaxWidth()) {
                                    val animatedProgress = animateFractionAsState(currentState.progress)

                                    LinearProgressIndicator(progress = animatedProgress)
                                    Text(
                                        text = "${(currentState.progress * 100F).roundToInt()}%",
                                        textAlign = Justification.CENTER
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LoadingStatusPage(
    modifier: Modifier = Modifier,
    title: String,
    description: String? = null,
    content: (@Composable () -> Unit)? = null
) {
    val pageWidget = remember { dev.datlag.kommons.adwaita.StatusPage() }
    val paintable = remember(pageWidget) { SpinnerPaintable(pageWidget) }

    ComposeNode<StatusPageNode, GtkApplier>(
        factory = { StatusPageNode(pageWidget) },
        update = {
            set(title) { this.widget.title = it }
            set(description) { this.widget.description = it }
            set(paintable) { this.widget.paintable = it }
            set(modifier) { this.applyModifier(it) }
        },
        content = {
            content?.invoke()
        }
    )
}

@Composable
private fun animateFractionAsState(targetFraction: Float): Float {
    var currentFraction by remember { mutableFloatStateOf(targetFraction) }

    LaunchedEffect(targetFraction) {
        if (targetFraction < currentFraction) {
            currentFraction = targetFraction
        }

        var lastFrameTimeNanos: Long? = null
        while (currentCoroutineContext().isActive && currentFraction < targetFraction) {
            withFrameNanos { frameTimeNanos ->
                if (lastFrameTimeNanos == null) {
                    lastFrameTimeNanos = frameTimeNanos
                    return@withFrameNanos
                }

                val dtMillis = (frameTimeNanos - (lastFrameTimeNanos ?: frameTimeNanos)) / 1_000_000.0
                lastFrameTimeNanos = frameTimeNanos

                val distance = targetFraction - currentFraction
                if (distance < 0.001) {
                    currentFraction = targetFraction
                    return@withFrameNanos
                }

                val minSpeedPerMs = 0.0001
                val dynamicSpeedPerMs = distance * 0.002
                val step = max(minSpeedPerMs, dynamicSpeedPerMs) * dtMillis

                currentFraction = min(targetFraction, currentFraction + step.toFloat())
            }
        }
    }

    return currentFraction
}