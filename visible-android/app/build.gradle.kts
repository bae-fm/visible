plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "fm.bae.visible"
    compileSdk = 35

    defaultConfig {
        applicationId = "fm.bae.visible"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
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
    }

    sourceSets {
        getByName("main") {
            java.srcDir("../../visible-bridge/kotlin-bindings")
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

    // Instrumented navigation test: TestNavHostController drives the real
    // browse/search graph to prove tapping a search result lands a back stack
    // that walks up the matched node's real ancestors.
    androidTestImplementation("androidx.test:core:1.6.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.navigation:navigation-testing:2.8.5")
}
