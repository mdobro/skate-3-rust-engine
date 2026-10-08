import org.gradle.process.ExecOperations
import javax.inject.Inject

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

fun git(vararg args: String): String = try {
    val p = ProcessBuilder("git", *args).directory(rootDir).redirectErrorStream(true).start()
    val out = p.inputStream.bufferedReader().readText().trim()
    if (p.waitFor() == 0) out else ""
} catch (e: Exception) {
    ""
}

fun prop(name: String): String? =
    (project.findProperty(name) as String?) ?: System.getenv(name)

val gitCount = git("rev-list", "--count", "HEAD").toIntOrNull() ?: 1
val gitSha = git("rev-parse", "--short", "HEAD").ifEmpty { "dev" }
val skipCargo = project.findProperty("skipCargo") == "true"
val keystore = prop("SKATE_KEYSTORE")

android {
    namespace = "com.skate3.engine"
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "com.skate3.engine"
        minSdk = 29
        targetSdk = 35
        versionCode = gitCount
        versionName = "0.1.0-$gitSha"
        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore)
                storePassword = prop("SKATE_KEYSTORE_PASSWORD")
                keyAlias = prop("SKATE_KEY_ALIAS")
                keyPassword = prop("SKATE_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            if (keystore != null) signingConfig = signingConfigs.getByName("release")
        }
    }

    buildFeatures { prefab = true }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }

    packaging { jniLibs { useLegacyPackaging = false } }
}

dependencies {
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core-ktx:1.15.0")
    testImplementation("junit:junit:4.13.2")
}

// Builds libskate_android.so (and libc++_shared.so) into src/main/jniLibs.
// Skip with -PskipCargo=true to build only the Kotlin side.
abstract class CargoNdk @Inject constructor(private val ops: ExecOperations) : DefaultTask() {
    @get:Input abstract val release: Property<Boolean>
    @get:Internal abstract val repoRoot: DirectoryProperty
    @get:Internal abstract val outDir: DirectoryProperty

    @TaskAction
    fun run() {
        val args = mutableListOf(
            "cargo", "ndk", "-t", "arm64-v8a", "--platform", "29", "--link-libcxx-shared",
            "-o", outDir.get().asFile.absolutePath,
            "build", "-p", "skate-android", "--no-default-features",
        )
        if (release.get()) args += "--release"
        ops.exec {
            workingDir = repoRoot.get().asFile
            commandLine(args)
        }
    }
}

for (variant in listOf("Debug", "Release")) {
    val t = tasks.register<CargoNdk>("cargoNdkBuild$variant") {
        group = "build"
        description = "Builds the Rust library for $variant with cargo-ndk"
        release.set(variant == "Release")
        repoRoot.set(rootProject.layout.projectDirectory.dir(".."))
        outDir.set(layout.projectDirectory.dir("src/main/jniLibs"))
        onlyIf { !skipCargo }
    }
    tasks.configureEach {
        if (name == "merge${variant}JniLibFolders" || name == "pre${variant}Build") dependsOn(t)
    }
}

tasks.register("cargoNdkBuild") {
    group = "build"
    description = "Builds the Rust library (debug and release variants as requested)"
    dependsOn(tasks.matching { it.name.startsWith("cargoNdkBuild") && it.name != "cargoNdkBuild" })
}
