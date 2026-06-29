package org.disastermesh.android.feature.home

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable

/** Minimal non-product screen used only to prove the Compose bootstrap. */
@Composable
fun BootstrapHome() {
    MaterialTheme {
        Surface {
            Text("DisasterMesh bootstrap")
        }
    }
}
