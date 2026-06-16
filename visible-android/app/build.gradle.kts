plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// versionName is the app's own `0.N` release line; versionCode the monotonic
// build number (the release workflow's run number). Both fall back to dev
// defaults so a plain local build needs no environment.
val visibleVersionName = System.getenv("VISIBLE_VERSION") ?: "0.1.0"
val visibleVersionCode = (System.getenv("VISIBLE_VERSION_CODE") ?: "1").toInt()
val releaseKeystore = System.getenv("ANDROID_KEYSTORE_FILE")

android {
    namespace = "fm.bae.visible"
    compileSdk = 35

    defaultConfig {
        applicationId = "fm.bae.visible"
        minSdk = 26
        targetSdk = 35
        versionCode = visibleVersionCode
        versionName = visibleVersionName
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        // Only wire the release signing config when CI supplies the keystore;
        // local `assembleRelease` then produces an unsigned APK rather than
        // failing, and debug installs are unaffected.
        if (releaseKeystore != null) {
            create("release") {
                storeFile = file(releaseKeystore)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            if (releaseKeystore != null) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
        // Generates BuildConfig.VERSION_NAME / VERSION_CODE for the Settings ▸
        // About line.
        buildConfig = true
    }

    sourceSets {
        getByName("main") {
            java.srcDir("../../visible-bridge/kotlin-bindings")
        }
    }

    testOptions {
        unitTests {
            // The extracted form-conversion helpers (NodeDetailLogic) log the
            // unparseable→null path through android.util.Log, which is a stub on
            // the JVM. Returning default values lets those real functions run
            // (Log.d returns 0) instead of throwing "not mocked".
            isReturnDefaultValues = true
        }
    }
}

dependencies {
    // These AndroidX versions are the highest compatible with the Kotlin 2.0.21
    // / AGP 8.7.3 / compileSdk 35 toolchain: above them, AndroidX pulls a Kotlin
    // 2.2+ stdlib the 2.0.21 compiler can't read, or requires compileSdk 36 /
    // AGP 8.9+.
    val composeBom = platform("androidx.compose:compose-bom:2025.01.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.navigation:navigation-compose:2.8.5")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("net.java.dev.jna:jna:5.15.0@aar")
    implementation("io.coil-kt.coil3:coil-compose:3.0.4")
    debugImplementation("androidx.compose.ui:ui-tooling")

    // Local JUnit unit tests for the pure app logic (shouldRemovePrevious and the
    // extracted Settings/Sharing/NodeDetail derivations). No Android framework, so
    // they run on the JVM under testDebugUnitTest.
    testImplementation("junit:junit:4.13.2")

    // Instrumented navigation test: TestNavHostController drives the real
    // browse/search graph to prove tapping a search result lands a back stack
    // that walks up the matched node's real ancestors.
    androidTestImplementation("androidx.test:core:1.6.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.navigation:navigation-testing:2.8.5")
}
