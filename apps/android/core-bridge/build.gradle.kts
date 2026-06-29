import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.Delete

plugins {
    alias(libs.plugins.android.library)
}

val rustWorkspace = rootProject.projectDir.resolve("../..").canonicalFile
val generatedJni = layout.projectDirectory.dir("src/main/jniLibs")

val buildRustAndroid by tasks.registering(Exec::class) {
    group = "build"
    description = "Builds the single Rust UniFFI library for all supported Android ABIs."
    workingDir(rustWorkspace)
    inputs.files(
        rustWorkspace.resolve("Cargo.toml"),
        rustWorkspace.resolve("Cargo.lock"),
        rustWorkspace.resolve("rust-toolchain.toml"),
    )
    inputs.dir(rustWorkspace.resolve("core"))
    outputs.dir(generatedJni)
    commandLine(
        providers.environmentVariable("CARGO").getOrElse("cargo"),
        "ndk",
        "--target", "arm64-v8a",
        "--target", "armeabi-v7a",
        "--target", "x86",
        "--target", "x86_64",
        "--platform", "26",
        "--output-dir", generatedJni.asFile.absolutePath,
        "build",
        "--locked",
        "--package", "mesh-ffi",
        "--release",
    )
}

tasks.named<Delete>("clean") {
    delete(generatedJni)
}

tasks.configureEach {
    if (name.startsWith("merge") && name.endsWith("JniLibFolders")) {
        dependsOn(buildRustAndroid)
    }
}

dependencies {
    implementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }
}
