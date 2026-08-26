package com.vant.Mio.Music

import android.content.Context
import android.net.wifi.WifiManager
import androidx.annotation.Keep

@Keep
object DlnaMulticastLock {
  private const val lockTag = "MioMusic:DLNA"
  private val monitor = Any()
  private var multicastLock: WifiManager.MulticastLock? = null

  @JvmStatic
  fun acquire(context: Context) {
    val applicationContext = context.applicationContext
    synchronized(monitor) {
      val lock = multicastLock ?: createLock(applicationContext)
      if (!lock.isHeld) {
        lock.acquire()
      }
    }
  }

  @JvmStatic
  fun release() {
    synchronized(monitor) {
      multicastLock?.takeIf { it.isHeld }?.release()
    }
  }

  private fun createLock(context: Context): WifiManager.MulticastLock {
    val wifiManager = context.getSystemService(Context.WIFI_SERVICE) as? WifiManager
      ?: throw IllegalStateException("Wi-Fi service is unavailable")
    return wifiManager.createMulticastLock(lockTag).also {
      it.setReferenceCounted(false)
      multicastLock = it
    }
  }
}
