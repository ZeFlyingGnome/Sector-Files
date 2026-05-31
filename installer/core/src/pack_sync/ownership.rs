use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Paths provided by the FIR packages rather than the GitHub repo, so the
/// GitHub overlay MUST NOT write to any path matching this list. The packages
/// only contribute sector files (`.sct`/`.ese`, renamed into `LFXX/Sectors`)
/// and the per-FIR `ICAO`/`NavData` folders (also mirrored into `LFXX`).
/// Everything else — `.prf`, settings, plugins (incl. CoFrance), Alias, etc. —
/// comes from GitHub.
///
/// Patterns are matched against the *destination* path inside the install
/// root, with forward slashes (e.g. "LFXX/Sectors/LFBB.sct").
pub const GNG_OWNED_PATHS: &[&str] = &[
    // Sector files (the package overlay renames them into LFXX/Sectors).
    "LFXX/Sectors",
    "LFXX/Sectors/**",
    // ICAO / NavData, both the LFXX mirror and the per-FIR sources.
    "LFXX/ICAO",
    "LFXX/ICAO/**",
    "LFXX/NavData",
    "LFXX/NavData/**",
    "LFBB/ICAO",
    "LFBB/ICAO/**",
    "LFBB/NavData",
    "LFBB/NavData/**",
    "LFEE/ICAO",
    "LFEE/ICAO/**",
    "LFEE/NavData",
    "LFEE/NavData/**",
    "LFFF/ICAO",
    "LFFF/ICAO/**",
    "LFFF/NavData",
    "LFFF/NavData/**",
    "LFMM/ICAO",
    "LFMM/ICAO/**",
    "LFMM/NavData",
    "LFMM/NavData/**",
    "LFRR/ICAO",
    "LFRR/ICAO/**",
    "LFRR/NavData",
    "LFRR/NavData/**",
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
        assert!(is_gng_owned(&set, &p("LFXX/NavData/airways.txt")));
    }

    #[test]
    fn github_provided_paths_are_not_gng_owned() {
        let set = gng_owned_set();
        assert!(!is_gng_owned(&set, &p("LFBB/ASR/something.asr")));
        assert!(!is_gng_owned(&set, &p("LFXX/Settings/Symbology.txt")));
        assert!(!is_gng_owned(&set, &p("LFXX/Plugins/CCAMS/CCAMS.dll")));
        // Now GitHub-provided: profiles, CoFrance, Alias, per-FIR settings.
        assert!(!is_gng_owned(&set, &p("LFBB/EGA Paris.prf")));
        assert!(!is_gng_owned(&set, &p("LFXX/Plugins/CoFrance/CoFranceLoader.dll")));
        assert!(!is_gng_owned(&set, &p("LFXX/Alias/Alias.txt")));
        assert!(!is_gng_owned(&set, &p("LFFF/Settings/LoginProfiles.txt")));
    }

    #[test]
    fn paths_with_backslashes_normalize() {
        let set = gng_owned_set();
        assert!(is_gng_owned(&set, &p("LFXX\\Sectors\\LFBB.sct")));
    }
}
