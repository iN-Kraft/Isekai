package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.navigation.component.DefaultScreen
import dev.datlag.isekai.navigation.model.DistroList
import dev.datlag.isekai.translation.DistroSelection
import dev.datlag.isekai.viewmodel.DistroViewModel
import dev.datlag.isekai.viewmodel.FileSelectViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.adwaita.ViewSwitcherPolicy
import dev.datlag.kommons.adwaita.compose.component.ActionRow
import dev.datlag.kommons.adwaita.compose.component.ButtonContent
import dev.datlag.kommons.adwaita.compose.component.Clamp
import dev.datlag.kommons.adwaita.compose.component.ExpanderRow
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.adwaita.compose.component.SplitButton
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
import dev.datlag.kommons.gtk.gio.FileInfo
import dev.datlag.kommons.gtk.gio.FileMeasureFlags
import dev.datlag.kommons.gtk.gio.measureDiskUsage

@Composable
fun DistroSelectionScreen(
    onSelected: (Screen.BlueprintScreen) -> Unit
) {
    val distroViewModel = kodeinViewModel<DistroViewModel>()
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
                    val distroList by distroViewModel.desktopDistros.collectAsState()

                    DistroListView(
                        distroList,
                        onSelect = onSelected,
                        onLocalSelect = { file ->
                            val filePath = file.peekPath()?.ifBlank { null } ?: file.getPath()?.ifBlank { null }
                            val fileSize = file.measureDiskUsage(FileMeasureFlags.NONE, null, null) { _, size, _, _ ->
                                size
                            }.getOrNull() ?: 0uL

                            if (filePath.isNullOrBlank()) {
                                snackbarHostState.showSnackbar(title = "Could not resolve path for: ${file.getBasename()}")
                            } else {
                                onSelected(Screen.BlueprintScreen.LocalFile(filePath, fileSize))
                            }
                        }
                    )
                }

                ViewStackPage(
                    name = "gaming",
                    title = GAMING,
                    iconName = "input-gaming-symbolic"
                ) {
                    val distroList by distroViewModel.gamingDistros.collectAsState()

                    DistroListView(
                        distroList,
                        onSelect = onSelected,
                        onLocalSelect = { file ->
                            val filePath = file.peekPath()?.ifBlank { null } ?: file.getPath()?.ifBlank { null }
                            val fileSize = file.measureDiskUsage(FileMeasureFlags.NONE, null, null) { _, size, _, _ ->
                                size
                            }.getOrNull() ?: 0uL

                            if (filePath.isNullOrBlank()) {
                                snackbarHostState.showSnackbar(title = "Could not resolve path for: ${file.getBasename()}")
                            } else {
                                onSelected(Screen.BlueprintScreen.LocalFile(filePath, fileSize))
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
                        when (distro) {
                            is DistroList.Distro.Standalone -> {
                                ActionRow(
                                    title = distro.name,
                                    subtitle = distro.tagline,
                                    suffix = {
                                        VariantDownloadButton(
                                            baseId = distro.id,
                                            baseConfig = distro.config,
                                            variants = distro.variants,
                                            distroName = distro.name,
                                            editionName = null,
                                            onSelect = onSelect
                                        )
                                    }
                                )
                            }
                            is DistroList.Distro.WithEditions -> {
                                ExpanderRow(
                                    title = distro.name,
                                    subtitle = distro.tagline
                                ) {
                                    distro.editions.forEach { edition ->
                                        ActionRow(
                                            title = edition.name,
                                            subtitle = edition.description,
                                            suffix = {
                                                VariantDownloadButton(
                                                    baseId = edition.id,
                                                    baseConfig = edition.config,
                                                    variants = edition.variants,
                                                    distroName = distro.name,
                                                    editionName = edition.name,
                                                    onSelect = onSelect
                                                )
                                            }
                                        )
                                    }
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
                            onClick = {
                                fileSelector.selectISO(currentWindow) { file ->
                                    file?.let { onLocalSelect(it) }
                                }
                            }
                        ) {
                            ButtonContent(label = BROWSE, iconName = "folder-open-symbolic")
                        }
                    }
                )
            }
        }
    }
}

@Composable
private fun VariantDownloadButton(
    baseId: String,
    baseConfig: DistroList.PublicConfig,
    variants: List<DistroList.Variant>,
    distroName: String,
    editionName: String?,
    onSelect: (Screen.BlueprintScreen.Download) -> Unit
) = with(DistroSelection) {
    if (variants.isEmpty()) {
        Button(
            modifier = Modifier.css("suggested-action").alignVertical(Align.CENTER),
            onClick = {
                onSelect(Screen.BlueprintScreen.Download(baseId, distroName, editionName))
            },
            enabled = baseConfig.available
        ) {
            ButtonContent(label = DOWNLOAD, iconName = "folder-download-symbolic")
        }
    } else {
        var selectedVariant by remember(variants) {
            mutableStateOf(variants.firstOrNull { it.config.available } ?: variants.firstOrNull())
        }

        SplitButton(
            modifier = Modifier.css("suggested-action").alignVertical(Align.CENTER),
            onClicked = {
                onSelect(Screen.BlueprintScreen.Download(selectedVariant?.id ?: baseId, distroName, editionName))
            },
            menu = {
                variants.forEach { variant ->
                    item(label = variant.name, onClick = { selectedVariant = variant })
                }
            }
        ) {
            ButtonContent(label = selectedVariant?.name ?: DOWNLOAD, iconName = "folder-download-symbolic")
        }
    }
}