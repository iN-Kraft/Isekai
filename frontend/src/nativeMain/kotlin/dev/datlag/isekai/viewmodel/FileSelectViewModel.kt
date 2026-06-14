package dev.datlag.isekai.viewmodel

import dev.datlag.kommons.gtk.FileDialog
import dev.datlag.kommons.gtk.FileFilter
import dev.datlag.kommons.gtk.Window
import dev.datlag.kommons.gtk.gio.File
import dev.datlag.kommons.gtk.gio.ListStore
import dev.datlag.kommons.gtk.native.GTK_TYPE_FILE_FILTER
import kotlinx.coroutines.CoroutineScope
import org.kodein.di.DirectDI

class FileSelectViewModel(
    override val directDI: DirectDI,
    viewModelScope: CoroutineScope
) : KodeinViewModel(directDI, viewModelScope) {

    private val isoFilter by lazy {
        val filter = FileFilter()
        filter.name = "Linux ISO"
        filter.addMimeType("application/x-iso9660-image")
        filter.addSuffix("iso")
        filter
    }

    private val filterStore by lazy {
        val store = ListStore(GTK_TYPE_FILE_FILTER)
        store.append(isoFilter)
        store
    }

    fun selectISO(window: Window?, callback: (File?) -> Unit) {
        val dialog = FileDialog()
        dialog.title = "Select ISO File"
        dialog.modal = true
        dialog.setFilters(filterStore)

        dialog.open(
            parent = window,
            cancellable = null
        ) { _, res ->
            val result = dialog.openFinish(res)
            callback(result.getOrNull())
        }
    }

}