use crate::fir::FirCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    #[serde(default)]
    pub controller_pack_dir: Option<PathBuf>,
    #[serde(default)]
    pub vatsim: VatsimCredentials,
    #[serde(default)]
    pub gng: GngSession,
    #[serde(default)]
    pub versions: InstalledVersions,
    #[serde(default)]
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatsimCredentials {
    #[serde(default)]
    pub cid: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub rating: String,
    #[serde(default)]
    pub real_name: String,
    #[serde(default = "default_true")]
    pub enable_rpc: bool,
}

impl Default for VatsimCredentials {
    fn default() -> Self {
        Self {
            cid: String::new(),
            password: String::new(),
            rating: "S1".into(),
            real_name: String::new(),
            enable_rpc: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GngSession {
    #[serde(default)]
    pub cookies: Vec<GngCookie>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub captured_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GngCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledVersions {
    #[serde(default)]
    pub installed_github_sha: Option<String>,
    #[serde(default)]
    pub installed_airac_cycle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    #[serde(default = "default_true")]
    pub apply_creds_after_sync: bool,
    #[serde(default = "default_firs")]
    pub selected_firs: Vec<FirCode>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_check_updates: true,
            apply_creds_after_sync: true,
            selected_firs: default_firs(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_firs() -> Vec<FirCode> {
    FirCode::ALL.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_round_trips_through_json() {
        let mut profile = Profile::default();
        profile.controller_pack_dir = Some(PathBuf::from("/tmp/pack"));
        profile.vatsim.cid = "1234567".into();
        profile.vatsim.password = "secret".into();
        profile.vatsim.rating = "C1".into();
        profile.versions.installed_github_sha = Some("abcdef1".into());
        profile.versions.installed_airac_cycle = Some("2605".into());

        let json = serde_json::to_string(&profile).unwrap();
        let back: Profile = serde_json::from_str(&json).unwrap();

        assert_eq!(back.controller_pack_dir, profile.controller_pack_dir);
        assert_eq!(back.vatsim.cid, "1234567");
        assert_eq!(back.vatsim.rating, "C1");
        assert_eq!(back.versions.installed_github_sha.as_deref(), Some("abcdef1"));
        assert!(back.preferences.auto_check_updates);
        assert_eq!(back.preferences.selected_firs.len(), 5);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let json = r#"{ "controller_pack_dir": "/tmp/x" }"#;
        let profile: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.controller_pack_dir, Some(PathBuf::from("/tmp/x")));
        assert_eq!(profile.vatsim.cid, "");
        assert_eq!(profile.vatsim.rating, "S1");
        assert!(profile.vatsim.enable_rpc);
        assert!(profile.preferences.auto_check_updates);
        assert_eq!(profile.preferences.selected_firs.len(), 5);
    }
}
