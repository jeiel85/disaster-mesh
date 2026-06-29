import com.android.build.api.dsl.LibraryExtension

buildscript {
    dependencies {
        // AGP 9 built-in Kotlin uses this higher stable KGP version.
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.0")
    }
}

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.kotlin.compose) apply false
}

allprojects {
    dependencyLocking {
        lockAllConfigurations()
    }
}

subprojects {
    pluginManager.withPlugin("com.android.library") {
        extensions.configure<LibraryExtension> {
            namespace = "org.disastermesh.android.${project.name.replace('-', '.')}"
            compileSdk = 37

            defaultConfig {
                minSdk = 26
            }

            compileOptions {
                sourceCompatibility = JavaVersion.VERSION_17
                targetCompatibility = JavaVersion.VERSION_17
            }

            lint {
                abortOnError = true
                checkReleaseBuilds = true
                warningsAsErrors = true
            }
        }
    }
}

tasks.register("verifyGoal0") {
    group = "verification"
    description = "Runs the repository-level Goal 0 verification entry points."
    dependsOn(
        ":app:assembleOfflineRelease",
        ":app:lintOfflineRelease",
        ":app:testOfflineReleaseUnitTest",
    )
}
