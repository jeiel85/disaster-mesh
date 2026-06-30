pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "DisasterMeshAndroid"

include(
    ":app",
    ":core-bridge",
    ":domain",
    ":security-keystore",
    ":transport-ble",
    ":service-relay",
    ":feature-onboarding",
    ":feature-home",
    ":feature-contacts",
    ":feature-conversation",
    ":feature-checkin",
    ":feature-sos",
    ":feature-relay-status",
    ":feature-diagnostics",
    ":feature-settings",
    ":test-fixtures",
)
