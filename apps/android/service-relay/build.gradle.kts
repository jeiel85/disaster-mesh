plugins {
    alias(libs.plugins.android.library)
}

dependencies {
    implementation(project(":domain"))
    // The application talks to the single relay/runtime boundary. These are
    // exported temporarily until the coordinator facade replaces direct type
    // exposure in Goal 5.
    api(project(":core-bridge"))
    api(project(":security-keystore"))
}
