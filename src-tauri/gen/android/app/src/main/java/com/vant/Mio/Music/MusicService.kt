package com.vant.Mio.Music

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.os.SystemClock
import android.support.v4.media.session.MediaSessionCompat
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.media.app.NotificationCompat.MediaStyle
import java.net.URL
import java.net.URLDecoder
import kotlin.concurrent.thread


class MusicService : Service() {
    companion object {
        const val TAG = "MusicService"
        const val CHANNEL_ID = "music_playback"
        const val NOTIFICATION_ID = 1001
        const val ACTION_PLAY = "com.vant.Mio.Music.PLAY"
        const val ACTION_PAUSE = "com.vant.Mio.Music.PAUSE"
        const val ACTION_NEXT = "com.vant.Mio.Music.NEXT"
        const val ACTION_PREV = "com.vant.Mio.Music.PREV"
        const val ACTION_STOP = "com.vant.Mio.Music.STOP"

        @Volatile
        var instance: MusicService? = null

        @Volatile private var pendingTitle = ""
        @Volatile private var pendingArtist = ""
        @Volatile private var pendingAlbum = ""
        @Volatile private var pendingDurationMs = 0L
        @Volatile private var pendingCoverUrl = ""

        // --- Static helpers called from Rust via JNI ---

        @JvmStatic
        fun start(context: Context) {
            val intent = Intent(context, MusicService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        @JvmStatic
        fun stop(context: Context) {
            context.stopService(Intent(context, MusicService::class.java))
        }

        @JvmStatic
        fun updateNowPlaying(
            title: String, artist: String, album: String,
            durationMs: Long, coverUrl: String
        ) {
            pendingTitle = title
            pendingArtist = artist
            pendingAlbum = album
            pendingDurationMs = durationMs
            pendingCoverUrl = coverUrl
            instance?.updateNowPlayingInternal(title, artist, album, durationMs, coverUrl)
        }

        @JvmStatic
        fun setPlaying(playing: Boolean) {
            instance?.setPlayingInternal(playing)
        }

        @JvmStatic
        fun updatePlaybackPosition(positionMs: Long, durationMs: Long) {
            instance?.updatePlaybackPositionInternal(positionMs, durationMs)
        }
    }

    fun isPlaying(): Boolean = nowPlaying

    private var mediaSession: MediaSession? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var audioFocusRequest: AudioFocusRequest? = null
    private var audioManager: AudioManager? = null
    private var coverBitmap: Bitmap? = null
    private var isForeground = false
    private var hasAudioFocus = false
    private var nowTitle = ""
    private var nowArtist = ""
    private var nowPlaying = false

    // --- Lifecycle ---

    override fun onCreate() {
        super.onCreate()
        instance = this
        createNotificationChannel()
        setupMediaSession()
        setupWakeLock()
        audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        if (pendingTitle.isNotEmpty()) {
            updateNowPlayingInternal(pendingTitle, pendingArtist, pendingAlbum, pendingDurationMs, pendingCoverUrl)
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        intent?.let { handleIntent(it) }
        if (!isForeground) startAsForeground()
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        releaseAudioFocus()
        releaseWakeLock()
        mediaSession?.release()
        mediaSession = null
        coverBitmap?.recycle()
        coverBitmap = null
        isForeground = false
        instance = null
    }

    // --- Notification ---

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID, "音乐播放", NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "显示当前播放的歌曲信息"
                setShowBadge(false)
                lockscreenVisibility = Notification.VISIBILITY_PUBLIC
            }
            (getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager)
                .createNotificationChannel(channel)
        }
    }

    private fun startAsForeground() {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        isForeground = true
    }

    private fun buildNotification(): Notification {
        val contentIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val prevIntent = PendingIntent.getService(this, 0,
            Intent(this, MusicService::class.java).setAction(ACTION_PREV),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)

        val playPauseAction = if (nowPlaying) ACTION_PAUSE else ACTION_PLAY
        val playPauseIcon = if (nowPlaying)
            android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play
        val playPauseText = if (nowPlaying) "暂停" else "播放"
        val playPauseIntent = PendingIntent.getService(this, 1,
            Intent(this, MusicService::class.java).setAction(playPauseAction),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)

        val nextIntent = PendingIntent.getService(this, 2,
            Intent(this, MusicService::class.java).setAction(ACTION_NEXT),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)

        val style = MediaStyle()
            .setMediaSession(mediaSession?.let {
                MediaSessionCompat.Token.fromToken(it.sessionToken)
            })
            .setShowActionsInCompactView(0, 1, 2)

        val displayTitle = when {
            nowTitle.isNotEmpty() && nowArtist.isNotEmpty() -> "$nowTitle - $nowArtist"
            nowTitle.isNotEmpty() -> nowTitle
            else -> "Mio Music"
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(displayTitle)
            .setContentText(nowArtist)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(contentIntent)
            .addAction(android.R.drawable.ic_media_previous, "上一首", prevIntent)
            .addAction(playPauseIcon, playPauseText, playPauseIntent)
            .addAction(android.R.drawable.ic_media_next, "下一首", nextIntent)
            .setStyle(style)
            .setOngoing(nowPlaying)
            .setOnlyAlertOnce(true)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .apply {
                if (lastDurationMs > 0) setProgress(lastDurationMs.toInt(), lastPositionMs.toInt(), false)
                coverBitmap?.let { setLargeIcon(it) }
            }
            .build()
    }

    private fun updateNotification() {
        if (!isForeground) return
        (getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager)
            .notify(NOTIFICATION_ID, buildNotification())
    }

    // --- MediaSession ---

    private fun setupMediaSession() {
        mediaSession = MediaSession(this, "MioMusic").apply {
            setCallback(object : MediaSession.Callback() {
                override fun onPlay() {
                    nativePlayerCommand("play")
                    updateNotificationState(true)
                }
                override fun onPause() {
                    nativePlayerCommand("pause")
                    updateNotificationState(false)
                }
                override fun onSkipToNext() { nativePlayerCommand("next") }
                override fun onSkipToPrevious() { nativePlayerCommand("prev") }
                override fun onSeekTo(pos: Long) {
                    lastPositionMs = pos
                    nativePlayerSeek(pos)
                    updatePlaybackPositionInternal(pos, lastDurationMs)
                    updateNotification()
                }
                override fun onStop() {
                    nativePlayerCommand("stop")
                    updateNotificationState(false)
                }
            })
            setFlags(MediaSession.FLAG_HANDLES_MEDIA_BUTTONS or
                     MediaSession.FLAG_HANDLES_TRANSPORT_CONTROLS)
            isActive = true
        }
    }

    private fun updateNowPlayingInternal(
        title: String, artist: String, album: String,
        durationMs: Long, coverUrl: String
    ) {
        nowTitle = title
        nowArtist = artist
        updateNotificationState(true)

        val displayTitle = if (artist.isNotEmpty()) "$title - $artist" else title
        val builder = MediaMetadata.Builder()
            .putString(MediaMetadata.METADATA_KEY_TITLE, displayTitle)
            .putString(MediaMetadata.METADATA_KEY_ARTIST, artist)
            .putString(MediaMetadata.METADATA_KEY_ALBUM, album)
            .putLong(MediaMetadata.METADATA_KEY_DURATION, durationMs)

        coverBitmap?.recycle()
        coverBitmap = null

        if (coverUrl.isNotEmpty()) {
            loadCoverAsync(coverUrl, builder)
        } else {
            mediaSession?.setMetadata(builder.build())
            updateNotification()
        }
    }

    private fun setPlayingInternal(playing: Boolean) {
        nowPlaying = playing
        if (playing) {
            acquireWakeLock()
            requestAudioFocus()
        } else {
            releaseWakeLock()
        }
        val state = if (playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED
        mediaSession?.setPlaybackState(
            PlaybackState.Builder()
                .setActions(
                    PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or
                    PlaybackState.ACTION_SKIP_TO_PREVIOUS or
                    PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_STOP
                )
                .setState(state, lastPositionMs, if (playing) 1.0f else 0.0f, SystemClock.elapsedRealtime())
                .build()
        )
        updateNotification()
    }

    private var lastPositionMs: Long = 0
    private var lastDurationMs: Long = 0

    private fun updatePlaybackPositionInternal(positionMs: Long, durationMs: Long) {
        lastPositionMs = positionMs
        lastDurationMs = durationMs
        val state = if (nowPlaying) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED
        mediaSession?.setPlaybackState(
            PlaybackState.Builder()
                .setActions(
                    PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or
                    PlaybackState.ACTION_SKIP_TO_PREVIOUS or
                    PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_STOP or
                    PlaybackState.ACTION_SEEK_TO
                )
                .setState(state, positionMs, if (nowPlaying) 1.0f else 0.0f, SystemClock.elapsedRealtime())
                .build()
        )
        // Update metadata duration if changed
        if (durationMs > 0) {
            mediaSession?.setMetadata(
                MediaMetadata.Builder(mediaSession?.controller?.metadata ?: MediaMetadata.Builder().build())
                    .putLong(MediaMetadata.METADATA_KEY_DURATION, durationMs)
                    .build()
            )
        }
    }

    private fun normalizeCoverUrl(url: String): String {
        val original = when {
            url.startsWith("http://imgproxy.localhost/") -> URLDecoder.decode(url.removePrefix("http://imgproxy.localhost/"), "UTF-8")
            url.startsWith("imgproxy://localhost/") -> URLDecoder.decode(url.removePrefix("imgproxy://localhost/"), "UTF-8")
            else -> url
        }
        return if (original.startsWith("http://")) "https://" + original.removePrefix("http://") else original
    }

    private fun loadCoverAsync(url: String, builder: MediaMetadata.Builder) {
        thread {
            try {
                val coverSource = normalizeCoverUrl(url)
                val conn = URL(coverSource).openConnection()
                conn.connectTimeout = 8000
                conn.readTimeout = 8000
                conn.setRequestProperty("User-Agent",
                    "Mozilla/5.0 (Linux; Android) AppleWebKit/537.36")
                val referer = coverSource.substringBefore('/', "").let { host ->
                    if (coverSource.startsWith("http://") || coverSource.startsWith("https://")) {
                        val protocol = coverSource.substringBefore("://")
                        val rest = coverSource.substringAfter("://")
                        "$protocol://${rest.substringBefore('/')}"
                    } else ""
                }
                if (referer.isNotEmpty()) {
                    conn.setRequestProperty("Referer", referer)
                    conn.setRequestProperty("Origin", referer)
                }
                val bmp = BitmapFactory.decodeStream(conn.getInputStream())
                if (bmp != null) {
                    coverBitmap = bmp
                    builder.putBitmap(MediaMetadata.METADATA_KEY_ART, bmp)
                }
            } catch (e: Exception) {
                Log.w(TAG, "Cover load failed: ${e.message}")
            }
            try {
                mediaSession?.setMetadata(builder.build())
                updateNotification()
            } catch (_: Exception) {}
        }
    }

    // --- WakeLock ---

    @Suppress("DEPRECATION")
    private fun setupWakeLock() {
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "MioMusic::Playback")
    }

    private fun acquireWakeLock() {
        wakeLock?.let { if (!it.isHeld) it.acquire(4 * 60 * 60 * 1000L) }
    }

    private fun releaseWakeLock() {
        wakeLock?.let { if (it.isHeld) it.release() }
    }

    // --- Audio Focus ---

    private fun requestAudioFocus() {
        if (hasAudioFocus) return
        val am = audioManager ?: return
        val result = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val attr = AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                .build()
            audioFocusRequest = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(attr)
                .setOnAudioFocusChangeListener { focus ->
                    if (focus == AudioManager.AUDIOFOCUS_LOSS && nowPlaying) {
                        updateNotificationState(false)
                        nativePlayerCommand("pause")
                    }
                }
                .build()
            audioFocusRequest?.let { am.requestAudioFocus(it) } ?: AudioManager.AUDIOFOCUS_REQUEST_FAILED
        } else {
            @Suppress("DEPRECATION")
            am.requestAudioFocus({ focus ->
                Log.d(TAG, "audioFocusChange focus=$focus nowPlaying=$nowPlaying")
                if (focus == AudioManager.AUDIOFOCUS_LOSS && nowPlaying) {
                    updateNotificationState(false)
                    nativePlayerCommand("pause")
                }
            }, AudioManager.STREAM_MUSIC, AudioManager.AUDIOFOCUS_GAIN)
        }
        hasAudioFocus = result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
    }

    private fun releaseAudioFocus() {
        val am = audioManager ?: return
        if (!hasAudioFocus) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            audioFocusRequest?.let { am.abandonAudioFocusRequest(it) }
        } else {
            @Suppress("DEPRECATION")
            am.abandonAudioFocus(null)
        }
        hasAudioFocus = false
    }

    // --- Intent handling ---

    private fun handleIntent(intent: Intent) {
        when (intent.action) {
            ACTION_PLAY -> { nativePlayerCommand("play"); updateNotificationState(true) }
            ACTION_PAUSE -> { nativePlayerCommand("pause"); updateNotificationState(false) }
            ACTION_NEXT -> nativePlayerCommand("next")
            ACTION_PREV -> nativePlayerCommand("prev")
            ACTION_STOP -> {
                nativePlayerCommand("stop")
                updateNotificationState(false)
                stopForeground(STOP_FOREGROUND_REMOVE)
                isForeground = false
                stopSelf()
            }
        }
    }

    private fun updateNotificationState(playing: Boolean) {
        nowPlaying = playing
        if (playing) {
            acquireWakeLock()
            requestAudioFocus()
        } else {
            releaseWakeLock()
        }
        val state = if (playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED
        mediaSession?.setPlaybackState(
            PlaybackState.Builder()
                .setActions(
                    PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or
                    PlaybackState.ACTION_SKIP_TO_PREVIOUS or
                    PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_STOP
                )
                .setState(state, lastPositionMs, if (playing) 1.0f else 0.0f, SystemClock.elapsedRealtime())
                .build()
        )
        updateNotification()
    }

    private external fun nativePlayerCommand(cmd: String)
    private external fun nativePlayerSeek(positionMs: Long)
}
