group = "com.vyrolink.vl_host"
version = "1.0-SNAPSHOT"

buildscript {
    val kotlinVersion = "2.4.0"
    repositories {
        google()
        mavenCentral()
    }

    dependencies {
        classpath("com.android.tools.build:gradle:9.1.0")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:$kotlinVersion")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

plugins {
    id("com.android.library")
}

android {
    namespace = "com.vyrolink.vl_host"

    compileSdk = 36

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            java.srcDirs("src/main/kotlin")
        }
    }

    defaultConfig {
        // Keep JNA and the Rust bindings intact in shrunk (release) app builds.
        consumerProguardFiles("consumer-rules.pro")
        // ADR-016: Android 12 (API 31) is the minimum; BLE permissions start there.
        minSdk = 31
    }

    testOptions {
        unitTests {
            isIncludeAndroidResources = true
            all {
                it.useJUnitPlatform()

                it.outputs.upToDateWhen { false }

                it.testLogging {
                    events("passed", "skipped", "failed", "standardOut", "standardError")
                    showStandardStreams = true
                }
            }
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

dependencies {
    // UniFFI's Kotlin bindings call the Rust library through JNA (Apache-2.0).
    implementation("net.java.dev.jna:jna:5.19.1@aar")
    // Pigeon's generated Kotlin uses coroutines for async calls.
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    // FileProvider for sharing an attachment (already present via CameraX).
    implementation("androidx.core:core:1.13.1")
    // QR pairing: CameraX preview + ZXing decoding on the device (no ML Kit, no network).
    implementation("androidx.camera:camera-camera2:1.6.1")
    implementation("androidx.camera:camera-lifecycle:1.6.1")
    implementation("androidx.camera:camera-view:1.6.1")
    implementation("com.google.zxing:core:3.5.4")
    // Same ID after a reinstall on this phone (ADR-038); device-only, no cloud copy.
    implementation("com.google.android.gms:play-services-auth-blockstore:16.4.0")
    testImplementation("org.jetbrains.kotlin:kotlin-test")
}
