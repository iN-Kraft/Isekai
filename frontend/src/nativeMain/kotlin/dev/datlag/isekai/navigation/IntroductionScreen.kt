package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ComposeNode
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.ipc.ConnectionState
import dev.datlag.isekai.module.tr
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.navigation.model.IntroSlide
import dev.datlag.isekai.viewmodel.ConnectionViewModel
import dev.datlag.isekai.viewmodel.SystemViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.CarouselIndicatorDots
import dev.datlag.kommons.adwaita.compose.component.ButtonContent
import dev.datlag.kommons.adwaita.compose.component.CarouselIndicatorDots
import dev.datlag.kommons.adwaita.compose.component.CarouselIndicatorDotsNode
import dev.datlag.kommons.adwaita.compose.component.CarouselState
import dev.datlag.kommons.adwaita.compose.component.StatusPage
import dev.datlag.kommons.adwaita.compose.component.rememberCarouselState
import dev.datlag.kommons.gtk.compose.GtkApplier
import dev.datlag.kommons.gtk.compose.component.Box
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.HorizontalDivider
import dev.datlag.kommons.gtk.compose.component.Row
import dev.datlag.kommons.gtk.compose.component.SeparatorNode
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import dev.datlag.kommons.gtk.compose.modifier.padding
import dev.datlag.kommons.gtk.compose.modifier.size
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO

@Composable
fun IntroductionScreen(
    onNavigateNext: (Screen) -> Unit
) = DefaultScreen {
    val connectionViewModel = kodeinViewModel<ConnectionViewModel>(dispatcher = Dispatchers.IO)
    val systemViewModel = kodeinViewModel<SystemViewModel>(dispatcher = Dispatchers.IO)
    
    val connectionState by connectionViewModel.connectionState.collectAsState()
    val report by systemViewModel.systemReport.collectAsState()

    var currentPage by remember { mutableIntStateOf(0) }
    val isLastPage = currentPage == IntroSlide.collection.lastIndex

    val onStartAction = {
        when (connectionState) {
            is ConnectionState.Connected if report?.isReady == true -> {
                onNavigateNext(Screen.Home)
            }

            is ConnectionState.Connected -> {
                onNavigateNext(Screen.SystemCheck)
            }

            else -> {
                onNavigateNext(Screen.Connection)
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxSize()
    ) {
        Box(
            modifier = Modifier
                .weight(1F)
                .fillMaxWidth(),
        ) {
            val slide = remember(currentPage) {
                IntroSlide.collection[currentPage]
            }

            StatusPage(
                modifier = Modifier.fillMaxSize(),
                icon = slide.icon,
                title = slide.title,
                description = slide.description
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth()
                .padding(24)
        ) {
            if (!isLastPage) {
                Button(
                    modifier = Modifier.css("flat", "pill"),
                    onClick = {
                        currentPage = IntroSlide.collection.lastIndex
                    },
                    label = tr("intro_skip", "Skip")
                )
            } else {
                Box(modifier = Modifier.size(width = 80)) {}
            }

            HorizontalDivider(modifier = Modifier.css("spacer").weight(1F))

            Button(
                modifier = Modifier.css("suggested-action", "pill"),
                onClick = {
                    if (isLastPage) onStartAction() else currentPage++
                }
            ) {
                if (isLastPage) {
                    ButtonContent(
                        label = tr("intro_start", "Start"),
                        iconName = "media-playback-start-symbolic"
                    )
                } else {
                    ButtonContent(
                        label = tr("intro_next", "Next"),
                        iconName = "go-next-symbolic"
                    )
                }
            }
        }
    }
}