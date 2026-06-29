package org.disastermesh.android

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.disastermesh.core.version
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class RustFacadeInstrumentationTest {
    @Test
    fun versionComesFromRustFacade() {
        assertEquals("0.1.0", version())
    }
}
