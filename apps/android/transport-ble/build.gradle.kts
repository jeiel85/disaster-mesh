plugins {
    alias(libs.plugins.android.library)
}

dependencies {
    implementation(project(":domain"))
    testImplementation(libs.junit4)
}
