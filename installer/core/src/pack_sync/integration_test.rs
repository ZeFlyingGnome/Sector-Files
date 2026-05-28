//! End-to-end test for the pack_sync planner + applier.
//!
//! Builds a fake GitHub repo tree and a fake GNG package tree inside a
//! tempdir, runs `plan` + `apply`, and asserts the resulting install tree
//! matches the expected layout from proposal.md.

use super::plan::{plan, PlanInputs};
use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_file(root: &Path, rel: &str, content: &str) {
    let dst = root.join(rel);
    fs::create_dir_all(dst.parent().unwrap()).unwrap();
    fs::write(dst, content).unwrap();
}

fn list_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        out.push(rel);
    }
    out.sort();
    out
}

fn build_fake_github_repo() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("vaccfr-Sector-Files-abcdef1");

    // Files the GitHub overlay SHOULD copy.
    write_file(&root, "LFBB/ASR/Tower.asr", "asr content\n");
    write_file(&root, "LFXX/Settings/Symbology.txt", "sym\n");
    write_file(&root, "LFXX/Plugins/CCAMS/CCAMS.dll", "ccams\n");
    write_file(&root, "LFXX/Plugins/CoFrance/CoFranceLoader.dll", "loader\n");
    write_file(&root, "LFBB/aeronav_copyright.txt", "(c) AeroNav\n");

    // Files the GitHub overlay SHOULD NOT copy (GNG-owned).
    write_file(&root, "LFXX/Sectors/STALE.txt", "ought-to-be-skipped\n");
    write_file(&root, "LFBB/ICAO/airports.txt", "GH should not write this\n");
    write_file(
        &root,
        "LFXX/Plugins/CoFrance/generated_state.dat",
        "GH should not write this\n",
    );

    let path = root.clone();
    (tmp, path)
}

fn build_fake_gng_package() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    // Sector + ese files (will be renamed to LFBB.sct / LFBB.ese).
    write_file(&root, "LFBB-Bordeaux-260301-0003.sct", "sct content\n");
    write_file(&root, "LFBB-Bordeaux-260301-0003.ese", "ese content\n");

    // ICAO + NavData under LFBB.
    write_file(&root, "LFBB/ICAO/airports.txt", "gng airports\n");
    write_file(&root, "LFBB/NavData/airways.txt", "navdata\n");

    // .prf at FIR root — should land in LFFF/Profiles/.
    write_file(&root, "LFFF/EGA Paris.prf", "prf content\n");

    // Per-FIR copyright.
    write_file(&root, "LFBB/aeronav_copyright.txt", "(c) AeroNav\n");

    // Legacy LFFM .sct — should be ignored.
    write_file(&root, "LFFM-Base-260301-0003.sct", "should be ignored\n");

    (tmp, root)
}

fn build_existing_install() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let install_root = tmp.path();

    // Pre-existing sector files at an older AIRAC cycle — should be moved to backup.
    write_file(install_root, "LFXX/Sectors/LFBB.sct", "old sct\n");
    write_file(install_root, "LFXX/Sectors/LFBB.ese", "old ese\n");
    write_file(install_root, "LFXX/Sectors/current_airac.txt", "2602\n");

    // A user-added custom file that should NOT be touched by the overlay.
    write_file(
        install_root,
        "LFXX/Plugins/CoFrance/user_settings.dat",
        "user state\n",
    );

    tmp
}

#[test]
fn end_to_end_sync_produces_expected_layout() {
    let (_gh_tmp, github_root) = build_fake_github_repo();
    let (_gng_tmp, gng_root) = build_fake_gng_package();
    let install_tmp = build_existing_install();
    let install_root = install_tmp.path();

    let gng_roots = vec![gng_root.clone()];
    let plan = plan(PlanInputs {
        github_root: Some(&github_root),
        gng_roots: &gng_roots,
        install_root,
        github_short_sha: Some("abcdef1".into()),
    })
    .unwrap();

    let summary = apply(install_root, &plan).unwrap();
    let files = list_files(install_root);

    // Sector files are at LFXX/Sectors with FIR-only names.
    assert!(files.contains(&"LFXX/Sectors/LFBB.sct".to_string()));
    assert!(files.contains(&"LFXX/Sectors/LFBB.ese".to_string()));

    // Old sector files were moved into LFXX/Sectors/Backup with the prev cycle suffix.
    assert!(
        files.iter().any(|f| f.starts_with("LFXX/Sectors/Backup/LFBB-2602")),
        "expected LFBB-2602 backup; got: {files:#?}"
    );

    // ICAO comes from GNG, NOT from GitHub.
    let icao_content = fs::read_to_string(install_root.join("LFBB/ICAO/airports.txt")).unwrap();
    assert_eq!(icao_content.trim(), "gng airports");

    // GitHub overlay landed where expected.
    assert!(files.contains(&"LFBB/ASR/Tower.asr".to_string()));
    assert!(files.contains(&"LFXX/Settings/Symbology.txt".to_string()));
    assert!(files.contains(&"LFXX/Plugins/CCAMS/CCAMS.dll".to_string()));

    // CoFrance loader exception came from GitHub.
    let loader = fs::read_to_string(install_root.join("LFXX/Plugins/CoFrance/CoFranceLoader.dll"))
        .unwrap();
    assert_eq!(loader.trim(), "loader");

    // User's pre-existing CoFrance state file is untouched.
    let user_state =
        fs::read_to_string(install_root.join("LFXX/Plugins/CoFrance/user_settings.dat")).unwrap();
    assert_eq!(user_state.trim(), "user state");

    // .prf was relocated into LFFF/Profiles/.
    assert!(files.contains(&"LFFF/Profiles/EGA Paris.prf".to_string()));

    // Per-FIR copyright preserved.
    assert!(files.contains(&"LFBB/aeronav_copyright.txt".to_string()));

    // AIRAC marker updated.
    let marker = fs::read_to_string(install_root.join("LFXX/Sectors/current_airac.txt")).unwrap();
    assert_eq!(marker.trim(), "2603");

    // Version marker written.
    assert!(files.contains(&".github/installer-version.txt".to_string()));
    let sha = fs::read_to_string(install_root.join(".github/installer-version.txt")).unwrap();
    assert_eq!(sha.trim(), "abcdef1");

    // LFFM sector file from GNG was NOT written.
    assert!(!files.iter().any(|f| f.contains("LFFM")));

    // Summary reports what we did.
    assert!(summary.files_written > 0);
    assert_eq!(summary.airac_cycle.as_deref(), Some("2603"));
    assert_eq!(summary.github_sha.as_deref(), Some("abcdef1"));
}

#[test]
fn second_run_is_a_no_op() {
    let (_gh_tmp, github_root) = build_fake_github_repo();
    let (_gng_tmp, gng_root) = build_fake_gng_package();
    let install_tmp = build_existing_install();
    let install_root = install_tmp.path();
    let gng_roots = vec![gng_root.clone()];

    let plan1 = plan(PlanInputs {
        github_root: Some(&github_root),
        gng_roots: &gng_roots,
        install_root,
        github_short_sha: Some("abcdef1".into()),
    })
    .unwrap();
    apply(install_root, &plan1).unwrap();

    // Second run with identical inputs — should write zero files.
    let plan2 = plan(PlanInputs {
        github_root: Some(&github_root),
        gng_roots: &gng_roots,
        install_root,
        github_short_sha: Some("abcdef1".into()),
    })
    .unwrap();
    let summary = apply(install_root, &plan2).unwrap();
    assert_eq!(summary.files_written, 0, "second run should be a no-op");
}
