//! App initialization: open a library and build its [`AppHandle`].

use std::path::PathBuf;
use std::sync::Arc;

use visible_core::app::bootstrap;

use crate::handle::AppHandle;
use crate::types::BridgeError;

#[uniffi::export]
pub fn init_app(data_dir: String, library_id: String) -> Result<Arc<AppHandle>, BridgeError> {
    configure_logging();

    Ok(Arc::new(AppHandle {
        app: bootstrap(&PathBuf::from(data_dir), library_id)?,
    }))
}

/// Build an `EnvFilter` from `RUST_LOG`, defaulting to "info" when the variable
/// is unset, and warning to stderr when it is set but unreadable or malformed.
fn env_filter() -> tracing_subscriber::EnvFilter {
    match std::env::var("RUST_LOG") {
        // Unset is the normal case — default silently.
        Err(std::env::VarError::NotPresent) => tracing_subscriber::EnvFilter::new("info"),
        // Present but not UTF-8: a real misconfiguration. The logger isn't up
        // yet, so stderr is the only sink.
        Err(std::env::VarError::NotUnicode(_)) => {
            eprintln!("warning: RUST_LOG is not valid UTF-8, falling back to \"info\"");
            tracing_subscriber::EnvFilter::new("info")
        }
        Ok(val) => tracing_subscriber::EnvFilter::try_new(&val).unwrap_or_else(|e| {
            eprintln!("warning: RUST_LOG={val:?} is malformed ({e}), falling back to \"info\"");
            tracing_subscriber::EnvFilter::new("info")
        }),
    }
}

// Install the global subscriber. `try_init` fails only when one is already
// installed (init_app ran earlier in this process), which is the expected
// idempotent case — the existing subscriber stays and receives this line.
fn install_subscriber(subscriber: impl tracing_subscriber::util::SubscriberInitExt) {
    if subscriber.try_init().is_err() {
        tracing::debug!("tracing subscriber already initialized; reusing the existing one");
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn configure_logging() {
    use tracing_subscriber::prelude::*;

    let oslog_layer = tracing_oslog::OsLogger::new("fm.bae.visible", "default");

    install_subscriber(
        tracing_subscriber::registry()
            .with(env_filter())
            .with(oslog_layer),
    );
}

#[cfg(target_os = "android")]
fn configure_logging() {
    use tracing_subscriber::prelude::*;

    let android_layer = tracing_android::layer("visible").unwrap();

    install_subscriber(
        tracing_subscriber::registry()
            .with(env_filter())
            .with(android_layer),
    );
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
fn configure_logging() {
    use tracing_subscriber::prelude::*;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_target(false)
        .with_file(true);

    install_subscriber(
        tracing_subscriber::registry()
            .with(env_filter())
            .with(fmt_layer),
    );
}
