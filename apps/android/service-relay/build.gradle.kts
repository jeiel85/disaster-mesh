plugins {
    alias(libs.plugins.android.library)
}

dependencies {
    implementation(project(":domain"))
    implementation(project(":core-bridge"))
}
