package org.disastermesh.android.feature.home

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource

/** Minimal non-product screen used only to prove the Compose bootstrap. */
@Composable
fun BootstrapHome() {
    MaterialTheme {
        Surface {
            Text(
                text = "DisasterMesh bootstrap\n\n" +
                    stringResource(R.string.security_limitation_long_term_key),
            )
        }
    }
}
