pub mod client;
pub mod login;
pub mod packages;

use crate::profile_store::{self, GngSession};
use serde::Serialize;
use tauri::AppHandle;

pub use client::{authed_client, GngAuthError, GNG_BASE_URL};
pub use login::open_login_window;

#[derive(Debug, Clone, Serialize)]
pub struct GngStatus {
    pub signed_in: bool,
    pub username: Option<String>,
}

pub async fn status(app: &AppHandle) -> anyhow::Result<GngStatus> {
    let profile = profile_store::load(app)?;
    Ok(status_from_session(&profile.gng))
}

pub fn status_from_session(session: &GngSession) -> GngStatus {
    let has_unexpired_cookie = session.cookies.iter().any(|c| match &c.expires_at {
        None => true,
        Some(ts) => chrono::DateTime::parse_from_rfc3339(ts)
            .map(|when| when.with_timezone(&chrono::Utc) > chrono::Utc::now())
            .unwrap_or(true),
    });
    GngStatus {
        signed_in: has_unexpired_cookie,
        username: session.username.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_store::GngCookie;

    fn cookie(expires_at: Option<&str>) -> GngCookie {
        GngCookie {
            name: "session".into(),
            value: "abc".into(),
            domain: "files.aero-nav.com".into(),
            path: "/".into(),
            expires_at: expires_at.map(str::to_string),
            secure: true,
            http_only: true,
        }
    }

    #[test]
    fn no_cookies_means_signed_out() {
        let session = GngSession::default();
        assert!(!status_from_session(&session).signed_in);
    }

    #[test]
    fn unexpired_cookie_means_signed_in() {
        let session = GngSession {
            cookies: vec![cookie(Some("2099-01-01T00:00:00Z"))],
            username: Some("test".into()),
            captured_at: None,
        };
        assert!(status_from_session(&session).signed_in);
    }

    #[test]
    fn expired_cookie_means_signed_out() {
        let session = GngSession {
            cookies: vec![cookie(Some("2000-01-01T00:00:00Z"))],
            username: None,
            captured_at: None,
        };
        assert!(!status_from_session(&session).signed_in);
    }

    #[test]
    fn missing_expiry_treated_as_session_cookie() {
        let session = GngSession {
            cookies: vec![cookie(None)],
            ..Default::default()
        };
        assert!(status_from_session(&session).signed_in);
    }
}
