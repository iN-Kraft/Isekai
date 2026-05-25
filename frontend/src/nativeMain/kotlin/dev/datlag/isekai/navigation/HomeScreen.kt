package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import dev.datlag.isekai.viewmodel.DiskViewModel
import dev.datlag.isekai.viewmodel.kodeinViewModel
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.padding

@Composable
fun HomeScreen() {
    val viewModel = kodeinViewModel<DiskViewModel>()
    val disks by viewModel.disks.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()
    val error by viewModel.error.collectAsState()

    LaunchedEffect(Unit) {
        viewModel.loadDisks()
    }

    Column(modifier = Modifier.padding(16)) {
        if (isLoading) {
            Text("Loading Disks...")
        } else if (error != null) {
            Text("Error: $error")
            Button(onClick = { viewModel.loadDisks() }) {
                Text("Retry")
            }
        } else {
            Text("Disks available: ${disks.size}")
            for (disk in disks) {
                Text("${disk.name} - ${disk.totalGb} GB")
            }
        }
    }
}