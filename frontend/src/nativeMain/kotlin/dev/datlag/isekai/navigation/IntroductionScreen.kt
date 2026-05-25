package dev.datlag.isekai.navigation

import androidx.compose.runtime.Composable
import dev.datlag.kommons.gtk.compose.component.Button
import dev.datlag.kommons.gtk.compose.component.Column
import dev.datlag.kommons.gtk.compose.component.Text
import dev.datlag.kommons.gtk.compose.modifier.Modifier
import dev.datlag.kommons.gtk.compose.modifier.fillMaxSize
import dev.datlag.kommons.gtk.compose.modifier.padding

@Composable
fun IntroductionScreen(onSkip: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize(),
    ) {
        Text("Welcome to Isekai")
        Text("Migrate to a new Linux Distribution!")
        Button(onClick = onSkip, modifier = Modifier.padding(top = 16)) {
            Text("Skip & Start")
        }
    }
}