package fm.bae.visible

import android.app.Application

/** Hosts the process-lifetime [AppSession]. */
class VisibleApp : Application() {
    val session = AppSession()
}
