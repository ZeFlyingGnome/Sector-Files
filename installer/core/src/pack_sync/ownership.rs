use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Paths managed by GNG / AeroNav rather than the GitHub repo. The GitHub
/// overlay MUST NOT write to any path matching this list. Direct port of
/// the original Python `GNG_ONLY_FILES` (scripts/installer/config.py).
///
/// Patterns are matched against the *destination* path inside the install
/// root, with forward slashes (e.g. "LFXX/Sectors/LFBB.sct").
pub const GNG_OWNED_PATHS: &[&str] = &[
    // Every sector file and folder under LFXX/Sectors.
    "LFXX/Sectors",
    "LFXX/Sectors/**",
    // Alias comes from GNG.
    "LFXX/Alias",
    "LFXX/Alias/**",
    // LFXX generated/navdata files.
    "LFXX/ICAO",
    "LFXX/ICAO/**",
    "LFXX/NavData",
    "LFXX/NavData/**",
    // CoFrance plugin generated files (the loader DLL is the exception
    // handled separately, see `is_cofrance_loader_exception`).
    "LFXX/Plugins/CoFrance",
    "LFXX/Plugins/CoFrance/**",
    // Settings backups.
    "LFXX/Settings/settings_backup",
    "LFXX/Settings/settings_backup/**",
    // Per-FIR ICAO/NavData and sensitive settings.
    "LFBB/ICAO",
    "LFBB/ICAO/**",
    "LFBB/NavData",
    "LFBB/NavData/**",
    "LFBB/Settings/LoginProfiles.txt",
    "LFBB/Settings/VoiceChannels.txt",
    "LFEE/ICAO",
    "LFEE/ICAO/**",
    "LFEE/NavData",
    "LFEE/NavData/**",
    "LFEE/Settings/LoginProfiles.txt",
    "LFEE/Settings/VoiceChannels.txt",
    "LFFF/ICAO",
    "LFFF/ICAO/**",
    "LFFF/NavData",
    "LFFF/NavData/**",
    "LFFF/Settings/LoginProfiles.txt",
    "LFFF/Settings/VoiceChannels.txt",
    "LFMM/ICAO",
    "LFMM/ICAO/**",
    "LFMM/NavData",
    "LFMM/NavData/**",
    "LFMM/Settings/LoginProfiles.txt",
    "LFMM/Settings/VoiceChannels.txt",
    "LFRR/ICAO",
    "LFRR/ICAO/**",
    "LFRR/NavData",
    "LFRR/NavData/**",
    "LFRR/Settings/LoginProfiles.txt",
    "LFRR/Settings/VoiceChannels.txt",
];

pub fn gng_owned_set() -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in GNG_OWNED_PATHS {
        builder.add(Glob::new(pattern).expect("invalid built-in pattern"));
    }
    builder.build().expect("invalid built-in glob set")
}

pub fn rel_path_str(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn is_gng_owned(set: &GlobSet, rel: &Path) -> bool {
    set.is_match(rel_path_str(rel))
}

/// The single explicit GitHub-overlay exception inside an otherwise GNG-owned
/// directory: the CoFrance loader DLL ships from the repo.
pub fn is_cofrance_loader_exception(rel: &Path) -> bool {
    rel_path_str(rel).eq_ignore_ascii_case(super::COFRANCE_LOADER_PATH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn sectors_directory_and_contents_are_owned() {
        let set = gng_owned_set();
        assert!(is_gng_owned(&set, &p("LFXX/Sectors")));
        assert!(is_gng_owned(&set, &p("LFXX/Sectors/LFBB.sct")));
        assert!(is_gng_owned(&set, &p("LFXX/Sectors/Backup/LFBB-2605.sct")));
    }

    #[test]
    fn per_fir_navdata_and_icao_are_owned() {
        let set = gng_owned_set();
        assert!(is_gng_owned(&set, &p("LFBB/ICAO/something.txt")));
        assert!(is_gng_owned(&set, &p("LFRR/NavData/sub/deep.dat")));
        assert!(is_gng_owned(&set, &p("LFFF/Settings/LoginProfiles.txt")));
    }

    #[test]
    fn repo_owned_paths_are_not_gng_owned() {
        let set = gng_owned_set();
        assert!(!is_gng_owned(&set, &p("LFBB/ASR/something.asr")));
        assert!(!is_gng_owned(&set, &p("LFXX/Settings/Symbology.txt")));
        assert!(!is_gng_owned(&set, &p("LFXX/Plugins/CCAMS/CCAMS.dll")));
        assert!(!is_gng_owned(&set, &p("LFFF/Profiles/EGA Paris.prf")));
    }

    #[test]
    fn cofrance_directory_owned_but_loader_is_exception() {
        let set = gng_owned_set();
        let loader = p("LFXX/Plugins/CoFrance/CoFranceLoader.dll");
        let other = p("LFXX/Plugins/CoFrance/generated.dat");
        assert!(is_gng_owned(&set, &loader));
        assert!(is_gng_owned(&set, &other));
        assert!(is_cofrance_loader_exception(&loader));
        assert!(!is_cofrance_loader_exception(&other));
    }

    #[test]
    fn paths_with_backslashes_normalize() {
        let set = gng_owned_set();
        assert!(is_gng_owned(&set, &p("LFXX\\Sectors\\LFBB.sct")));
    }
}
