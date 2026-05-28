# GNG (files.aero-nav.com) integration

Two open problems are tracked here, both gated on real-account access:

## 1. Cookie capture (task 3.6)

The login flow is straightforward: open the AeroNav URL in a Tauri `WebviewWindow` and let the user complete VATSIM OAuth. The hard part is reading the resulting cookies back into Rust so `reqwest` can use them for headless downloads.

Tauri 2 does **not** expose a cross-platform cookie-jar API in stable Rust. The path forward depends on whether the AeroNav session cookie is `HttpOnly`:

- **If not `HttpOnly`** (best case): read it via `document.cookie` from an injected script, post the result back to Rust through a Tauri event or invoke. The plumbing for this is half-written in `login.rs::extract_cookies_from_window` and needs a JS bridge added under `installer/src/` — a small script in the auth window that calls `window.__TAURI__.event.emit("gng:cookies", document.cookie)` on `DOMContentLoaded`.

- **If `HttpOnly`** (likely case): use the platform-specific cookie manager:
  - **Windows (WebView2)**: `ICoreWebView2CookieManager::GetCookies` via the `webview2-com` crate. Accessible through `window.with_webview()` on Tauri 2.
  - **macOS (WKWebView)**: `WKHTTPCookieStore.getAllCookies(_:)` via `objc2-web-kit`. Accessible through `window.with_webview()`.
  - **Linux (WebKit2GTK)**: `webkit_website_data_manager_get_cookie_manager()` + `webkit_cookie_manager_get_cookies()` via `webkit2gtk-rs`. Accessible through `window.with_webview()`.

The current code falls back to the JS approach and returns an empty cookie list if it can't read them — the UI will then show "Sign in again" rather than silently failing.

**What to do once you have a real account**:

1. Sign in once with the WebView open.
2. Open the WebView's devtools, check whether the session cookie has the `HttpOnly` flag.
3. Implement whichever path applies (JS bridge or platform-specific).
4. Commit a **redacted** cookie fixture (`tests/fixtures/gng_cookie_redacted.json`) for future regression tests.

## 2. Package URL discovery (task 5.4)

The current Python installer expects the user to manually download `.zip`/`.7z` packages from `files.aero-nav.com` and feed them in. For the Tauri rewrite we need to download them programmatically. Two paths:

- **Option A — direct URLs**: discover whether each FIR has a stable "latest" URL (e.g. `files.aero-nav.com/api/packages/LFBB/latest`). Inspect network traffic in the WebView when the user clicks a download link manually; if a stable URL exists, hit it from `reqwest` with the captured cookies.

- **Option B — page interception**: if there's no stable API, render the file listing page in the WebView and intercept the download event (Tauri 2: `window.on_download`). Stash the downloaded archive in a tempdir and pass it to `pack_sync::apply`.

Document the discovered approach here once verified, and implement it in `installer/src-tauri/src/gng/packages.rs` (file does not yet exist — create when filling §5.3).
