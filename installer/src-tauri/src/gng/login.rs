use crate::profile_store::{self, GngCookie, GngSession};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LOGIN_WINDOW_LABEL: &str = "gng-auth";
pub const LOGIN_URL: &str = "https://files.aero-nav.com/login";
pub const POST_LOGIN_HOST: &str = "files.aero-nav.com";

pub async fn open_login_window(app: &AppHandle) -> anyhow::Result<()> {
    if let Some(existing) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        existing.set_focus().ok();
        return Ok(());
    }

    let url = url::Url::parse(LOGIN_URL)?;
    let app_handle = app.clone();
    let window_label = LOGIN_WINDOW_LABEL.to_string();

    WebviewWindowBuilder::new(app, LOGIN_WINDOW_LABEL, WebviewUrl::External(url))
        .title("Sign in to AeroNav")
        .inner_size(900.0, 760.0)
        .resizable(true)
        .on_navigation(move |new_url| {
            let host = new_url.host_str().unwrap_or("");
            let path = new_url.path();
            // After successful OAuth the user lands back on files.aero-nav.com
            // on a non-login path. That's our signal to try and capture cookies.
            if host == POST_LOGIN_HOST
                && !path.to_ascii_lowercase().contains("login")
                && !path.to_ascii_lowercase().contains("oauth")
            {
                let app = app_handle.clone();
                let label = window_label.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(window) = app.get_webview_window(&label) {
                        if let Err(error) = capture_session(&app, &window).await {
                            tracing::warn!(?error, "failed to capture GNG session cookies");
                        }
                    }
                });
            }
            true
        })
        .build()?;

    Ok(())
}

async fn capture_session(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
) -> anyhow::Result<()> {
    let cookies = extract_cookies_from_window(window).await?;

    if cookies.is_empty() {
        tracing::debug!("no cookies captured yet — likely still navigating");
        return Ok(());
    }

    let mut profile = profile_store::load(app)?;
    profile.gng = GngSession {
        cookies,
        username: profile.gng.username,
        captured_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    profile_store::save(app, &profile)?;

    app.emit("gng:session-captured", ()).ok();
    window.close().ok();
    Ok(())
}

/// Extract cookies for files.aero-nav.com from the auth window.
///
/// IMPLEMENTATION NOTE (Phase 3.6 follow-up): Tauri 2 does not expose a
/// fully cross-platform cookie API in stable Rust as of writing. This
/// function is a stub returning an empty list; the real implementation
/// needs to use one of two approaches depending on whether AeroNav's
/// session cookie is `HttpOnly`. See `gng/README.md` for the platform-
/// specific hooks (`window.with_webview(...)` on Windows/macOS/Linux).
async fn extract_cookies_from_window(
    _window: &tauri::WebviewWindow,
) -> anyhow::Result<Vec<GngCookie>> {
    Ok(parse_document_cookie_string("", POST_LOGIN_HOST))
}

/// Parse the string form of `document.cookie` (no metadata: no Domain, no
/// Expires, no HttpOnly flag) into our `GngCookie` shape. Domain/Path are
/// assumed; expiry is unknown.
pub fn parse_document_cookie_string(raw: &str, default_domain: &str) -> Vec<GngCookie> {
    raw.split(';')
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (name, value) = trimmed.split_once('=')?;
            Some(GngCookie {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
                domain: default_domain.to_string(),
                path: "/".to_string(),
                expires_at: None,
                secure: true,
                http_only: false,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_document_cookie_string() {
        let raw = "session=abc123; theme=dark";
        let cookies = parse_document_cookie_string(raw, POST_LOGIN_HOST);
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name, "session");
        assert_eq!(cookies[0].value, "abc123");
        assert_eq!(cookies[0].domain, POST_LOGIN_HOST);
        assert!(!cookies[0].http_only);
    }

    #[test]
    fn empty_string_produces_no_cookies() {
        assert!(parse_document_cookie_string("", POST_LOGIN_HOST).is_empty());
    }
}
