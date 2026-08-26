package com.vant.Mio.Music

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  companion object {
    init { System.loadLibrary("mio_lib") }
  }
  private external fun initAndroidContext(activity: android.app.Activity)
  private var webView: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    initAndroidContext(this)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
  }

  override fun onPause() {
    super.onPause()
    if (MusicService.instance?.isPlaying() == true) {
      webView?.onResume()
    }
  }
}
