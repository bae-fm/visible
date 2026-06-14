package fm.bae.visible.ui

/**
 * The pure derivations behind [SettingsViewModel]'s status line and connect
 * gate, with no [uniffi.visible_bridge.AppHandle] or observable state — the view
 * model reads its fields and the bridge records, then delegates the decision
 * here. Pulled out so the derivations are exercised directly, without standing up
 * a view model.
 */
object SettingsLogic {
    /**
     * The one-line settings status: the in-flight connect first, then the
     * configured/ready state, with the pending delete count appended when there
     * is work queued.
     */
    fun statusLine(
        working: Boolean,
        configured: Boolean,
        ready: Boolean,
        pendingDeletes: ULong,
    ): String {
        if (working) return "Connecting…"
        if (!configured) return "Not connected"
        val base = if (ready) "Synced" else "Connected (starting…)"
        return if (pendingDeletes > 0u) "$base · $pendingDeletes to delete" else base
    }

    /**
     * Whether the connect button has the minimum required fields. Bucket, region,
     * and both keys are required; endpoint and prefix are optional. A connect
     * already in flight ([working]) also disables it.
     */
    fun canConnect(
        bucket: String,
        region: String,
        accessKey: String,
        secretKey: String,
        working: Boolean,
    ): Boolean =
        !working && bucket.isNotEmpty() && region.isNotEmpty() &&
            accessKey.isNotEmpty() && secretKey.isNotEmpty()

    /**
     * Map an optional S3 form box (endpoint or key prefix) to its absence: trim
     * surrounding whitespace, and treat a blank or whitespace-only box as null so
     * core receives None, never "".
     */
    fun optionalField(text: String): String? = text.trim().ifEmpty { null }
}
