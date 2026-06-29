import com.android.build.api.variant.HostTestBuilder

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "org.disastermesh.android"
    compileSdk = 37

    defaultConfig {
        applicationId = "org.disastermesh.android"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {
            abiFilters += setOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }

    flavorDimensions += "distribution"
    productFlavors {
        create("offline") {
            dimension = "distribution"
        }
        create("fieldTest") {
            dimension = "distribution"
            applicationIdSuffix = ".fieldtest"
            versionNameSuffix = "-fieldtest"
        }
        create("dev") {
            dimension = "distribution"
            applicationIdSuffix = ".dev"
            versionNameSuffix = "-dev"
        }
    }

    buildTypes {
        debug {
            isDebuggable = true
        }
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        abortOnError = true
        checkReleaseBuilds = true
        warningsAsErrors = true
        // Target 36 and Gradle 9.4.1 are deliberate compatibility baselines
        // from the approved design and AGP 9.2 compatibility table.
        disable += setOf("OldTargetApi", "AndroidGradlePluginVersion")
    }

    packaging {
        jniLibs {
            excludes += setOf("**/armeabi/**", "**/mips/**", "**/mips64/**")
        }
    }
}

androidComponents {
    beforeVariants(selector().all()) { variant ->
        val distribution = variant.productFlavors
            .firstOrNull { it.first == "distribution" }
            ?.second
        val enabled = when (distribution to variant.buildType) {
            "offline" to "release",
            "fieldTest" to "release",
            "dev" to "debug" -> true
            else -> false
        }
        variant.enable = enabled
        variant.hostTests.values
            .filter { it.type == HostTestBuilder.UNIT_TEST_TYPE }
            .forEach { it.enable = enabled }
    }
}

dependencies {
    implementation(project(":feature-onboarding"))
    implementation(project(":feature-home"))
    implementation(project(":feature-contacts"))
    implementation(project(":feature-conversation"))
    implementation(project(":feature-checkin"))
    implementation(project(":feature-sos"))
    implementation(project(":feature-relay-status"))
    implementation(project(":feature-diagnostics"))
    implementation(project(":service-relay"))

    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.activity.compose)

    testImplementation(libs.junit4)
    androidTestImplementation(project(":core-bridge"))
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.ext.junit)
}
