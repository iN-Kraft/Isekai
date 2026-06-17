package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.navigation.model.DistroList
import dev.datlag.isekai.translation.DistroSelection
import dev.datlag.isekai.viewmodel.FileSelectViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.ViewSwitcherPolicy
import dev.datlag.kommons.adwaita.compose.component.ActionRow
import dev.datlag.kommons.adwaita.compose.component.Clamp
import dev.datlag.kommons.adwaita.compose.component.ExpanderRow
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.adwaita.compose.component.ViewStack
import dev.datlag.kommons.adwaita.compose.component.ViewStackPage
import dev.datlag.kommons.adwaita.compose.component.ViewSwitcher
import dev.datlag.kommons.adwaita.compose.component.rememberViewStackState
import dev.datlag.kommons.gtk.Align
import dev.datlag.kommons.gtk.compose.LocalWindow
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.LinkButton
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.alignVertical
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.gio.File

@Composable
fun DistroSelectionScreen(
    onSelected: (Screen.BlueprintScreen) -> Unit
) {
    var selectedTab by remember { mutableStateOf("desktop") }
    val viewStackState = rememberViewStackState()

    DefaultScreen(
        translation = DistroSelection,
        title = { ViewSwitcher(state = viewStackState, policy = ViewSwitcherPolicy.WIDE) }
    ) { snackbarHostState ->
        Column(modifier = Modifier.fillMaxSize()) {
            ViewStack(
                modifier = Modifier.fillMaxSize(),
                selectedPage = selectedTab,
                onPageChange = { selectedTab = it },
                state = viewStackState
            ) {
                ViewStackPage(
                    name = "desktop",
                    title = DESKTOP,
                    iconName = "computer-symbolic"
                ) {
                    DistroListView(
                        DistroList.desktop,
                        onSelect = onSelected,
                        onLocalSelect = { file ->
                            val filePath = file.peekPath()?.ifBlank { null } ?: file.getPath()?.ifBlank { null }

                            if (filePath.isNullOrBlank()) {
                                snackbarHostState.showSnackbar(title = "Could not resolve path for: ${file.getBasename()}")
                            } else {
                                onSelected(Screen.BlueprintScreen.LocalFile(filePath))
                            }
                        }
                    )
                }

                ViewStackPage(
                    name = "gaming",
                    title = GAMING,
                    iconName = "input-gaming-symbolic"
                ) {
                    DistroListView(
                        DistroList.gaming,
                        onSelect = onSelected,
                        onLocalSelect = { file ->
                            val filePath = file.peekPath()?.ifBlank { null } ?: file.getPath()?.ifBlank { null }

                            if (filePath.isNullOrBlank()) {
                                snackbarHostState.showSnackbar(title = "Could not resolve path for: ${file.getBasename()}")
                            } else {
                                onSelected(Screen.BlueprintScreen.LocalFile(filePath))
                            }
                        }
                    )
                }
            }
        }
    }
}

@Composable
private fun DistroListView(
    distroGroups: List<DistroList>,
    onSelect: (Screen.BlueprintScreen.Download) -> Unit,
    onLocalSelect: (File) -> Unit
) = with(DistroSelection) {
    Clamp(modifier = Modifier.fillMaxSize(), maximumSize = 800) {
        PreferencesPage(modifier = Modifier.fillMaxSize()) {
            distroGroups.forEach { distroGroup ->
                PreferencesGroup(
                    title = distroGroup.groupName
                ) {
                    distroGroup.groupList.forEach { distro ->
                        if (distro.editions.isEmpty()) {
                            ActionRow(
                                title = distro.name,
                                subtitle = distro.tagline,
                                suffix = {
                                    Button(
                                        modifier = Modifier.css("suggested-action").alignVertical(Align.CENTER),
                                        label = DOWNLOAD,
                                        onClick = {
                                            onSelect(
                                                Screen.BlueprintScreen.Download(
                                                    name = distro.name,
                                                    edition = null
                                                )
                                            )
                                        }
                                    )
                                }
                            )
                        } else {
                            ExpanderRow(
                                title = distro.name,
                                subtitle = distro.tagline
                            ) {
                                distro.editions.forEach { edition ->
                                    ActionRow(
                                        title = edition.name,
                                        subtitle = edition.description,
                                        suffix = {
                                            Button(
                                                modifier = Modifier.css("suggested-action").alignVertical(Align.CENTER),
                                                label = DOWNLOAD,
                                                onClick = {
                                                    onSelect(
                                                        Screen.BlueprintScreen.Download(
                                                            name = distro.name,
                                                            edition = edition.name
                                                        )
                                                    )
                                                }
                                            )
                                        }
                                    )
                                }
                            }
                        }
                    }
                }
            }
            PreferencesGroup(
                title = "Other"
            ) {
                val fileSelector = kodeinViewModel<FileSelectViewModel>()

                ActionRow(
                    title = UNSURE_TITLE,
                    subtitle = UNSURE_TEXT,
                    suffix = {
                        LinkButton(
                            modifier = Modifier.alignVertical(Align.CENTER),
                            uri = "https://distrosea.com/",
                            label = "DistroSea"
                        )
                    }
                )
                ActionRow(
                    title = LOCAL_TITLE,
                    subtitle = LOCAL_TEXT,
                    suffix = {
                        val currentWindow = LocalWindow.current

                        Button(
                            modifier = Modifier.alignVertical(Align.CENTER),
                            label = BROWSE,
                            onClick = {
                                fileSelector.selectISO(currentWindow) { file ->
                                    file?.let { onLocalSelect(it) }
                                }
                            }
                        )
                    }
                )
            }
        }
    }
}