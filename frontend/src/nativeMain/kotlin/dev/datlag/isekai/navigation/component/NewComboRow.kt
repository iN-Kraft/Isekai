package dev.datlag.isekai.navigation.component

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ComposeNode
import androidx.compose.runtime.remember
import dev.datlag.kommons.adwaita.ComboRow
import dev.datlag.kommons.adwaita.compose.component.ComboRowNode
import dev.datlag.kommons.gtk.compose.GtkApplier
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.gio.ListModel

/**
 * TODO: Patch upstream
 */
@Composable
fun NewComboRow(
    selected: Int,
    onSelectedChange: (Int) -> Unit,
    modifier: Modifier = Modifier,
    title: String? = null,
    subtitle: String? = null,
    model: ListModel? = null,
    enableSearch: Boolean = false,
    useSubtitle: Boolean = false,
    enabled: Boolean = true,
    visible: Boolean = true
) {
    val row = remember { ComboRow() }

    ComposeNode<ComboRowNode, GtkApplier>(
        factory = { ComboRowNode(row) },
        update = {
            set(selected) { this.updateSelectedSafely(it) }
            set(onSelectedChange) { this.onSelectedChange = it }
            set(enabled) { this.widget.sensitive = it }
            set(title) { this.widget.title = it ?: "" }
            set(subtitle) { this.widget.subtitle = it ?: "" }
            set(model) {
                this.isUpdatingFromCompose = true
                this.widget.useSubtitle = useSubtitle
                this.widget.model = it
                this.updateSelectedSafely(selected)
                this.isUpdatingFromCompose = false
            }
            set(enableSearch) { this.widget.enableSearch = it }
            set(useSubtitle) { this.widget.useSubtitle = it }
            set(modifier) { this.applyModifier(it) }
            set(visible) { this.widget.visible = it }
        }
    )
}