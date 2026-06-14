package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import dev.datlag.isekai.navigation.model.DistroList
import dev.datlag.kommons.adwaita.ViewSwitcherPolicy
import dev.datlag.kommons.adwaita.compose.component.ActionRow
import dev.datlag.kommons.adwaita.compose.component.Clamp
import dev.datlag.kommons.adwaita.compose.component.ExpanderRow
import dev.datlag.kommons.adwaita.compose.component.PreferencesGroup
import dev.datlag.kommons.adwaita.compose.component.PreferencesPage
import dev.datlag.kommons.adwaita.compose.component.Scaffold
import dev.datlag.kommons.adwaita.compose.component.TopAppBar
import dev.datlag.kommons.adwaita.compose.component.ViewStack
import dev.datlag.kommons.adwaita.compose.component.ViewStackPage
import dev.datlag.kommons.adwaita.compose.component.ViewSwitcher
import dev.datlag.kommons.adwaita.compose.component.rememberViewStackState
import dev.datlag.kommons.gtk.ActionBar
import dev.datlag.kommons.gtk.Align
import dev.datlag.kommons.gtk.compose.component.Box
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.LinkButton
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.alignVertical
import dev.datlag.kommons.gtk.compose.modifier.css
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.fillMaxWidth
import dev.datlag.kommons.gtk.compose.modifier.padding

@Composable
fun OSSelectionScreen(
    onSelected: () -> Unit,
    onLocalSelected: () -> Unit
) {
    var selectedTab by remember { mutableStateOf("desktop") }
    val viewStackState = rememberViewStackState()

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        topBar = {
            TopAppBar(
                modifier = Modifier.fillMaxWidth(),
                title = {
                    ViewSwitcher(state = viewStackState, policy = ViewSwitcherPolicy.WIDE)
                }
            )
        }
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            ViewStack(
                modifier = Modifier.fillMaxSize(),
                selectedPage = selectedTab,
                onPageChange = { selectedTab = it },
                state = viewStackState
            ) {
                ViewStackPage(
                    name = "desktop",
                    title = "Desktop",
                    iconName = "computer-symbolic"
                ) {
                    DistroListView(DistroList.desktop, onSelect = {}, onLocalSelect = {})
                }

                ViewStackPage(
                    name = "gaming",
                    title = "Gaming",
                    iconName = "input-gaming-symbolic"
                ) {
                    DistroListView(DistroList.gaming, onSelect = {}, onLocalSelect = {})
                }
            }
        }
    }
}

@Composable
private fun DistroListView(
    distroGroups: List<DistroList>,
    onSelect: (DistroList.Distro) -> Unit,
    onLocalSelect: () -> Unit
) {
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
                                onActivated = { onSelect(distro) },
                                suffix = {
                                    Button(
                                        modifier = Modifier.css("suggested-action").alignVertical(Align.CENTER),
                                        label = "Select",
                                        onClick = { onSelect(distro) }
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
                                        onActivated = { onSelect(distro) },
                                        suffix = {
                                            Button(
                                                modifier = Modifier.css("suggested-action").alignVertical(Align.CENTER),
                                                label = "Select",
                                                onClick = { onSelect(distro) }
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
                ActionRow(
                    title = "Unsure what to pick?",
                    subtitle = "Try them out online",
                    suffix = {
                        LinkButton(
                            modifier = Modifier.alignVertical(Align.CENTER),
                            uri = "https://distrosea.com/",
                            label = "DistroSea"
                        )
                    }
                )
                ActionRow(
                    title = "Select local ISO",
                    subtitle = "Bring your own distribution",
                    suffix = {
                        Button(
                            modifier = Modifier.alignVertical(Align.CENTER),
                            label = "Browse",
                            onClick = onLocalSelect
                        )
                    }
                )
            }
        }
    }
}