package fm.bae.visible

import android.app.Application
import io.crates.keyring.Keyring
import uniffi.visible_bridge.initKeyring
import uniffi.visible_bridge.setCaCertDir

/** Hosts the process-lifetime [AppSession] and wires cloud-sync startup. */
class VisibleApp : Application() {
    val session = AppSession()

    override fun onCreate() {
        super.onCreate()
        // The TLS stack (the S3 client, via rustls-native-certs) can't find CA
        // roots on Android's default POSIX probe paths, so cloud connections fail
        // to verify without this. Point it at the OS trust store so the platform
        // owns CA trust + updates. conscrypt's dir is the Play-updatable one (API
        // 30+); the /system dir is the fallback on older devices.
        setCaCertDir("/apex/com.android.conscrypt/cacerts:/system/etc/security/cacerts")
        // Hand the Android keyring store its Context before initKeyring(), so the
        // Rust side can reach the keystore for the sync identity + encryption key.
        Keyring.initializeNdkContext(this)
        initKeyring()
    }
}
