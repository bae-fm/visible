import OSLog

extension Logger {
    /// A logger under visible's subsystem — the same subsystem the Rust core
    /// logs to via `tracing-oslog`, so app and core lines group together.
    static func visible(_ category: String) -> Logger {
        Logger(subsystem: "fm.bae.visible", category: category)
    }
}
