//! Persistence layer for the user profile. Data types live in
//! `controller_pack_core::profile_types`; this module wraps them with
//! `tauri-plugin-store` load/save and the controller-pack-detection helper.

pub use controller_pack_core::profile_types::{
    InstalledVersions, Preferences, Profile, VatsimCredentials,
};
use controller_pack_core::AreaCode;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

pub const STORE_FILE: &str = "profile.json";
pub const PROFILE_KEY: &str = "profile";

pub fn load<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Profile> {
    let store = app.store(STORE_FILE)?;
    Ok(match store.get(PROFILE_KEY) {
        Some(value) => serde_json::from_value(value).unwrap_or_default(),
        None => Profile::default(),
    })
}

pub fn save<R: Runtime>(app: &AppHandle<R>, profile: &Profile) -> anyhow::Result<()> {
    let store = app.store(STORE_FILE)?;
    store.set(PROFILE_KEY, serde_json::to_value(profile)?);
    store.save()?;
    Ok(())
}

pub fn looks_like_controller_pack(path: &Path) -> bool {
    let has_lfxx = path.join("LFXX").is_dir();
    // Any area folder counts, LFFM included — it is not a FIR, but a pack that
    // only installed the military area is still a controller pack.
    let has_any_area = AreaCode::ALL
        .iter()
        .any(|area| path.join(area.as_str()).is_dir());
    has_lfxx && has_any_area
}

pub fn detect_pack_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    if looks_like_controller_pack(&cwd) {
        Some(cwd)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn looks_like_controller_pack_requires_lfxx_and_one_area() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        assert!(!looks_like_controller_pack(root));
        std::fs::create_dir_all(root.join("LFXX")).unwrap();
        assert!(!looks_like_controller_pack(root));
        std::fs::create_dir_all(root.join("LFBB")).unwrap();
        assert!(looks_like_controller_pack(root));
    }

    #[test]
    fn lffm_only_pack_is_recognised() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("LFXX")).unwrap();
        std::fs::create_dir_all(root.join("LFFM")).unwrap();
        assert!(looks_like_controller_pack(root));
    }
}
