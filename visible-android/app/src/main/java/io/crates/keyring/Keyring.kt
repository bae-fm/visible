package io.crates.keyring

import android.content.Context

/**
 * Bridge to the Android keyring store compiled into visible-core (the
 * `android-native-keyring-store` crate, linked into `libvisible_bridge.so`).
 * `initializeNdkContext` hands the Rust side the Android [Context] it needs for
 * the keystore, and must run before `initKeyring()`. The JNI symbol lives in the
 * bridge library, so load that rather than a separate `.so`.
 */
class Keyring {
    companion object {
        init {
            System.loadLibrary("visible_bridge")
        }

        external fun initializeNdkContext(context: Context)
    }
}
