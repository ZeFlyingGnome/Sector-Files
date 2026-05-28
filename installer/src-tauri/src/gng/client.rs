use crate::profile_store::{GngCookie, GngSession};
use reqwest::{cookie::Jar, header, redirect, Client, ClientBuilder, Url};
use std::sync::Arc;
use thiserror::Error;

pub const GNG_BASE_URL: &str = "https://files.aero-nav.com";

#[derive(Debug, Error)]
pub enum GngAuthError {
    #[error("GNG session expired or not signed in")]
    SessionExpired,
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
}

pub fn user_agent() -> String {
    format!(
        "vaccfr-controller-pack-installer/{}",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn authed_client(session: &GngSession) -> Result<Client, GngAuthError> {
    let jar = Arc::new(Jar::default());
    let base = Url::parse(GNG_BASE_URL)?;
    for cookie in &session.cookies {
        let cookie_str = format_set_cookie(cookie);
        jar.add_cookie_str(&cookie_str, &base);
    }

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_str(&user_agent()).unwrap(),
    );

    Ok(ClientBuilder::new()
        .cookie_provider(jar)
        .default_headers(headers)
        .redirect(redirect::Policy::custom(|attempt| {
            // Treat redirects to the login page as session expiry, not as
            // successful redirects: stop the chain so the caller can detect it.
            if attempt
                .url()
                .path()
                .to_ascii_lowercase()
                .contains("login")
            {
                attempt.stop()
            } else if attempt.previous().len() > 10 {
                attempt.error("too many redirects")
            } else {
                attempt.follow()
            }
        }))
        .build()?)
}

fn format_set_cookie(c: &GngCookie) -> String {
    let mut s = format!("{}={}; Domain={}; Path={}", c.name, c.value, c.domain, c.path);
    if c.secure {
        s.push_str("; Secure");
    }
    if c.http_only {
        s.push_str("; HttpOnly");
    }
    if let Some(exp) = &c.expires_at {
        s.push_str(&format!("; Expires={}", exp));
    }
    s
}

/// Inspect a response to decide whether the session is still valid.
/// Used by callers to convert ambiguous HTTP results into `SessionExpired`.
pub fn classify_response(resp: &reqwest::Response) -> Result<(), GngAuthError> {
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(GngAuthError::SessionExpired);
    }
    let url = resp.url();
    if url.path().to_ascii_lowercase().contains("login") {
        return Err(GngAuthError::SessionExpired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_includes_crate_version() {
        let ua = user_agent();
        assert!(ua.starts_with("vaccfr-controller-pack-installer/"));
        assert!(ua.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn format_set_cookie_includes_all_attributes() {
        let cookie = GngCookie {
            name: "session".into(),
            value: "abc123".into(),
            domain: "files.aero-nav.com".into(),
            path: "/".into(),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            secure: true,
            http_only: true,
        };
        let s = format_set_cookie(&cookie);
        assert!(s.contains("session=abc123"));
        assert!(s.contains("Domain=files.aero-nav.com"));
        assert!(s.contains("Secure"));
        assert!(s.contains("HttpOnly"));
        assert!(s.contains("Expires=2099-01-01T00:00:00Z"));
    }

    #[test]
    fn authed_client_can_be_constructed() {
        let session = GngSession {
            cookies: vec![GngCookie {
                name: "session".into(),
                value: "abc".into(),
                domain: "files.aero-nav.com".into(),
                path: "/".into(),
                expires_at: None,
                secure: true,
                http_only: true,
            }],
            ..Default::default()
        };
        assert!(authed_client(&session).is_ok());
    }
}
