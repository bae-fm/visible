//! App initialization: open a library and build its [`AppHandle`].

use std::sync::Arc;

use visible_core::app::{bootstrap, RunningApp};

use crate::handle::AppHandle;
use crate::types::BridgeError;

#[uniffi::export]
pub fn init_app(library_id: String) -> Result<Arc<AppHandle>, BridgeError> {
    configure_logging();

    let RunningApp { runtime, inventory } = bootstrap(library_id)?;

    Ok(Arc::new(AppHandle { runtime, inventory }))
}

/// Build an `EnvFilter` from `RUST_LOG`, defaulting to "info" when the variable
/// is unset, and warning to stderr when it is set but malformed.
fn env_filter() -> tracing_subscriber::EnvFilter {
    match std::env::var("RUST_LOG") {
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
        Ok(val) => tracing_subscriber::EnvFilter::try_new(&val).unwrap_or_else(|e| {
            eprintln!("warning: RUST_LOG={val:?} is malformed ({e}), falling back to \"info\"");
            tracing_subscriber::EnvFilter::new("info")
        }),
    }
}

// Install the global subscriber, ignoring the "already initialized" error —
// the documented use of `try_init`.
fn install_subscriber(subscriber: impl tracing_subscriber::util::SubscriberInitExt) {
    let _ = subscriber.try_init();
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
