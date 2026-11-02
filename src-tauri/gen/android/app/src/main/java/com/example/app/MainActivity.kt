package com.example.app

import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.ColorUtils
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.insets.GradientProtection
import androidx.core.view.insets.ProtectionLayout

// The page's colour behind the status bar, fading out downwards: Android
// draws the bar see-through over an app (edge-to-edge, SDK 35 on), and the
// page scrolled on under the clock. Google's protection for it, the
// counterpart of the iPhone's own blur there (an islands app on this stack,
// 2026-10-29). The colour is the page's own `theme-color`, never a copy
// here (SURFACE_HEX, src/views/layout.rs).
class MainActivity : TauriActivity() {
  private var webView: WebView? = null
  private var statusBar: GradientProtection? = null

  // Transparent, as Android draws it, until the first page says its colour.
  private var color = Color.TRANSPARENT

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
  }

  // wry hands its webview to setContentView: it goes inside the layout that
  // draws the protection over it, the webview still getting every inset
  // (the gradient takes none, so the site's `--safe-area-top` stays). A
  // protection of its own each time: one belongs to one layout only.
  override fun setContentView(view: View?) {
    if (view == null) {
      super.setContentView(view)
      return
    }
    (view.parent as? ViewGroup)?.removeView(view)
    val protection = GradientProtection(WindowInsetsCompat.Side.TOP, color)
    statusBar = protection
    val layout = ProtectionLayout(this, listOf(protection))
    layout.addView(view, ViewGroup.LayoutParams(MATCH_PARENT, MATCH_PARENT))
    super.setContentView(layout)
  }

  // A turn or a day-night switch: the page's media queries answer anew once
  // the webview has taken its new size and scheme, a moment later.
  override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    webView?.postDelayed({ paintStatusBar() }, RESETTLE_MS)
  }

  // Called by the app's Rust at every page load (src-tauri/src/lib.rs) and
  // above: the page's `theme-color` tag whose media query matches now, its
  // colour given to the protection. No tag, or anything but #rrggbb, leaves
  // the colour as it was.
  fun paintStatusBar() {
    runOnUiThread {
      webView?.evaluateJavascript(THEME_COLOR) { answer ->
        val hex = answer?.trim('"') ?: return@evaluateJavascript
        if (HEX.matches(hex)) {
          color = Color.parseColor(hex)
          statusBar?.setColor(color)
          // The clock and icons dark on a light colour and light on a dark
          // one: Android sets them by the system's scheme once, at the start,
          // and a day-night switch left them dark on a dark page.
          WindowCompat.getInsetsController(window, window.decorView)
            .isAppearanceLightStatusBars = ColorUtils.calculateLuminance(color) > 0.5
        }
      }
    }
  }

  private companion object {
    const val RESETTLE_MS = 300L
    const val THEME_COLOR =
      "(() => { for (const tag of document.querySelectorAll('meta[name=\"theme-color\"]')) " +
        "{ if (!tag.media || matchMedia(tag.media).matches) return tag.content; } return null; })()"
    val HEX = Regex("^#[0-9a-fA-F]{6}$")
  }
}
