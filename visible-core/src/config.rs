//! Keyring setup. coven reads the cloud-sync identity (the user keypair) and the
//! per-library encryption key from the OS keyring through `keyring-core`; the
//! host installs the platform-native store and names coven's keyring service
//! once at startup, before any keyring access.

use tracing::{info, warn};

/// Install the platform keyring store and set coven's keyring service name.
///
/// coven namespaces every key entry under the host app's identity, which must be
/// set once before any keyring access — coven's getters panic otherwise.
/// "visible" keeps visible's coven key entries from colliding with any other
/// coven-based app on the same machine (e.g. bae). Set-once, so it is safe to
/// run through every startup path. Apps call this before opening a session.
///
/// On macOS the protected data store is used with iCloud key sync enabled, so a
/// returning user's encryption key restores from iCloud Keychain (when they have
/// it on); if that store can't be created the keyring falls back to the
/// local-only protected store, which holds keys on this machine but does not sync
/// them across the user's devices.
pub fn init_keyring() {
    coven::keys::set_keyring_service("visible");

    #[cfg(target_os = "macos")]
    {
        use std::collections::HashMap;
        let config = HashMap::from([("cloud-sync", "true")]);
        match apple_native_keyring_store::protected::Store::new_with_configuration(&config) {
            Ok(store) => {
                keyring_core::set_default_store(store);
                info!("Keyring initialized (protected store, iCloud sync enabled)");
            }
            Err(e) => {
                warn!("Failed to create protected keyring store: {e}, falling back to local");
                match apple_native_keyring_store::protected::Store::new() {
                    Ok(store) => {
                        keyring_core::set_default_store(store);
                        info!("Keyring initialized (protected store, local only)");
                    }
                    Err(e) => warn!("Failed to create local protected keyring store: {e}"),
                }
            }
        }
    }

    #[cfg(target_os = "ios")]
    {
        match apple_native_keyring_store::protected::Store::new() {
            Ok(store) => {
                keyring_core::set_default_store(store);
                info!("Keyring initialized (protected store)");
            }
            Err(e) => warn!("Failed to create protected keyring store: {e}"),
        }
    }

    #[cfg(target_os = "android")]
    {
        match android_native_keyring_store::Store::new() {
            Ok(store) => {
                keyring_core::set_default_store(store);
                info!("Keyring initialized (Android keystore)");
            }
            Err(e) => warn!("Failed to create Android keyring store: {e}"),
        }
    }
}

/// Install an in-memory keyring store and set coven's keyring service, for tests.
///
/// coven's `KeyService` reads and writes the keyring instead of the environment,
/// and its getters panic unless the service is set. Tests don't run
/// `init_keyring` (which installs the OS store and prompts), so this stands in:
/// an in-memory store for the OS keyring, service set to "visible" to match
/// production. Genuinely set-once — the mock store is installed on the first call
/// and kept for the rest of the process. Replacing it on a later call would wipe
/// entries other parallel tests already wrote (one process-global namespace);
/// entries stay isolated by library id instead.
#[cfg(test)]
pub fn install_test_keyring() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        keyring_core::set_default_store(
            keyring_core::mock::Store::new().expect("create mock keyring store"),
        );
        coven::keys::set_keyring_service("visible");
    });
}
