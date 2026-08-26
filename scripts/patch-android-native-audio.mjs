import { cpSync, existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const mainPath = 'src-tauri/gen/android/app/src/main/java/com/vant/Mio/Music/MainActivity.kt'
const servicePath = 'src-tauri/gen/android/app/src/main/java/com/vant/Mio/Music/MusicService.kt'
const iconSourceDir = 'src-tauri/icons/android'
const resDir = 'src-tauri/gen/android/app/src/main/res'

if (!existsSync(mainPath)) {
  console.warn('[patch-android] MainActivity.kt not found, skipping native audio patch')
} else {
  let main = readFileSync(mainPath, 'utf8')

  // Step 1: import WebView
  if (!main.includes('import android.webkit.WebView')) {
    main = main.replace(
      'import android.os.Bundle\n',
      'import android.os.Bundle\nimport android.webkit.WebView\n',
    )
  }

  // Step 2: companion object with native lib
  if (!main.includes('companion object {')) {
    main = main.replace(
      'class MainActivity : TauriActivity() {',
      'class MainActivity : TauriActivity() {\n  companion object {\n    init { System.loadLibrary("mio_lib") }\n  }',
    )
  }

  // Step 3: JNI external declaration + webView field
  if (!main.includes('external fun initAndroidContext')) {
    main = main.replace(
      'class MainActivity : TauriActivity() {',
      'class MainActivity : TauriActivity() {\n  private external fun initAndroidContext(activity: android.app.Activity)',
    )
  }
  if (!main.includes('private var webView: WebView? = null')) {
    main = main.replace(
      '  private external fun initAndroidContext(activity: android.app.Activity)\n',
      '  private external fun initAndroidContext(activity: android.app.Activity)\n  private var webView: WebView? = null\n',
    )
  }

  // Step 4: inject initAndroidContext(this) into onCreate
  if (!main.includes('initAndroidContext(this)')) {
    main = main.replace(
      '  override fun onCreate(savedInstanceState: Bundle?) {\n    enableEdgeToEdge()',
      '  override fun onCreate(savedInstanceState: Bundle?) {\n    initAndroidContext(this)\n    enableEdgeToEdge()',
    )
  }

  // Step 5: add onWebViewCreate + onPause for background playback
  if (!main.includes('override fun onWebViewCreate')) {
    main = main.replace(
      '    super.onCreate(savedInstanceState)\n  }\n}',
      '    super.onCreate(savedInstanceState)\n  }\n\n  override fun onWebViewCreate(webView: WebView) {\n    this.webView = webView\n  }\n\n  override fun onPause() {\n    super.onPause()\n    if (MusicService.instance?.isPlaying() == true) {\n      webView?.onResume()\n    }\n  }\n}',
    )
  }

  writeFileSync(mainPath, main)
  console.log('[patch-android] Patched MainActivity.kt')
}

if (!existsSync(servicePath)) {
  console.warn('[patch-android] MusicService.kt not found, skipping playback state patch')
} else {
  let service = readFileSync(servicePath, 'utf8')
  if (!service.includes('fun isPlaying(): Boolean')) {
    service = service.replace(
      '    private var mediaSession: MediaSession? = null\n',
      '    fun isPlaying(): Boolean = nowPlaying\n\n    private var mediaSession: MediaSession? = null\n',
    )
  }
  writeFileSync(servicePath, service)
  console.log('[patch-android] Patched MusicService.kt')
}

if (!existsSync(iconSourceDir)) {
  console.warn('[patch-android] Android icon source directory not found, skipping icon patch')
} else if (!existsSync(resDir)) {
  console.warn('[patch-android] Generated Android res directory not found, skipping icon patch')
} else {
  cpSync(iconSourceDir, resDir, { recursive: true, force: true })
  console.log(`[patch-android] Copied Android icons from ${iconSourceDir} to ${resDir}`)
}
