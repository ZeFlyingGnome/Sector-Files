use super::airac::parse_gng_sector_filename;
use super::ownership::{gng_owned_set, is_cofrance_loader_exception, is_gng_owned};
use super::{COPYRIGHT_FILE, CURRENT_AIRAC_FILE, SECTORS_SUBPATH, SECTOR_BACKUP_DIRNAME};
use crate::fir::FirCode;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOp {
    /// Copy a file from one location to another. Used for the GitHub overlay
    /// and for non-sector GNG files (e.g. ICAO, NavData).
    Copy { src: PathBuf, dst: PathBuf },
    /// Move an existing installed sector file to the backup directory before
    /// writing a new one in its place.
    BackupSector { src: PathBuf, dst: PathBuf },
    /// Write a sector file to its canonical location, renamed to <FIR>.<ext>.
    WriteSector { src: PathBuf, dst: PathBuf, fir: FirCode, ext: SectorExt },
    /// Move a `.prf` file into the FIR's Profiles/ subdirectory.
    MoveProfile { src: PathBuf, dst: PathBuf },
    /// Write/overwrite a marker text file with a given string value.
    WriteText { dst: PathBuf, value: String },
    /// Ensure a directory exists.
    EnsureDir { path: PathBuf },
    /// Delete a legacy file (e.g. root-level `.sct` from old layouts).
    DeleteLegacy { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum SectorExt {
    Sct,
    Ese,
}

impl SectorExt {
    pub fn as_str(self) -> &'static str {
        match self {
            SectorExt::Sct => "sct",
            SectorExt::Ese => "ese",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "sct" => Some(SectorExt::Sct),
            "ese" => Some(SectorExt::Ese),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct SyncPlan {
    pub ops: Vec<FileOp>,
    pub detected_airac: Option<String>,
    pub previous_airac: Option<String>,
    pub github_short_sha: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SyncSummary {
    pub github_sha: Option<String>,
    pub airac_cycle: Option<String>,
    pub files_written: usize,
    pub files_skipped: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct PlanInputs<'a> {
    pub github_root: Option<&'a Path>,
    pub gng_roots: &'a [PathBuf],
    pub install_root: &'a Path,
    pub github_short_sha: Option<String>,
    /// FIRs the user wants installed. Top-level folders for any FIR *not* in
    /// this list are skipped by the GitHub overlay and removed from the
    /// install root. `LFXX` (the shared base) is always kept.
    pub selected_firs: &'a [FirCode],
}

/// The first path segment parsed as a FIR code, if it is one. `LFXX` and other
/// non-FIR roots return `None`.
fn top_level_fir(rel: &Path) -> Option<FirCode> {
    rel.components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .and_then(|s| s.parse::<FirCode>().ok())
}

pub fn plan(inputs: PlanInputs<'_>) -> anyhow::Result<SyncPlan> {
    let mut plan = SyncPlan {
        github_short_sha: inputs.github_short_sha.clone(),
        previous_airac: read_current_airac(inputs.install_root),
        ..Default::default()
    };
    let gng_set = gng_owned_set();

    // 0) Remove top-level folders for FIRs the user did not select.
    for fir in FirCode::ALL {
        if !inputs.selected_firs.contains(&fir) {
            let dir = inputs.install_root.join(fir.as_str());
            if dir.is_dir() {
                plan.ops.push(FileOp::DeleteLegacy { path: dir });
            }
        }
    }

    // 1) Plan GitHub-source operations (overlay), skipping GNG-owned paths and
    //    any unselected FIR's folder.
    if let Some(github_root) = inputs.github_root {
        plan_github_overlay(
            github_root,
            inputs.install_root,
            &gng_set,
            inputs.selected_firs,
            &mut plan,
        );
    }

    // 2) Plan GNG-source operations across all extracted packages. Sector
    //    writes are collected separately so they can be ordered AFTER their
    //    matching BackupSector ops in step 3.
    let mut sector_writes: Vec<FileOp> = Vec::new();
    let mut sector_targets: BTreeSet<(FirCode, SectorExt)> = BTreeSet::new();
    for gng_root in inputs.gng_roots {
        plan_gng_overlay(
            gng_root,
            inputs.install_root,
            &mut plan,
            &mut sector_writes,
            &mut sector_targets,
        );
    }

    // Filter no-op sector writes (src content matches the file already at dst).
    sector_writes.retain(|op| match op {
        FileOp::WriteSector { src, dst, fir, ext } => {
            if files_equal(src, dst) {
                sector_targets.remove(&(*fir, *ext));
                false
            } else {
                true
            }
        }
        _ => true,
    });

    // 3) Backup existing sector files for sectors that will actually be overwritten.
    if !sector_targets.is_empty() {
        let previous_airac = plan.previous_airac.clone();
        plan_sector_backups(
            inputs.install_root,
            &sector_targets,
            previous_airac.as_deref(),
            &mut plan,
        );
    }

    // 4) Now append the sector writes AFTER backups.
    plan.ops.extend(sector_writes);

    // 5) AIRAC marker — only write if we have a parsed cycle.
    if let Some(cycle) = &plan.detected_airac {
        plan.ops.push(FileOp::WriteText {
            dst: inputs.install_root.join(CURRENT_AIRAC_FILE),
            value: cycle.clone(),
        });
    } else if !sector_targets.is_empty() {
        plan.warnings.push(
            "Sector files present but no AIRAC cycle could be parsed; current_airac.txt left unchanged"
                .into(),
        );
    }

    // 6) GitHub installer-version marker, if we synced from GitHub.
    if let Some(sha) = &inputs.github_short_sha {
        plan.ops.push(FileOp::WriteText {
            dst: inputs.install_root.join(super::INSTALLER_VERSION_FILE),
            value: sha.clone(),
        });
    }

    Ok(plan)
}

fn files_equal(a: &Path, b: &Path) -> bool {
    let (am, bm) = match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(am), Ok(bm)) => (am, bm),
        _ => return false,
    };
    if am.len() != bm.len() {
        return false;
    }
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(ab), Ok(bb)) => ab == bb,
        _ => false,
    }
}

fn plan_github_overlay(
    github_root: &Path,
    install_root: &Path,
    gng_set: &globset::GlobSet,
    selected_firs: &[FirCode],
    plan: &mut SyncPlan,
) {
    for entry in WalkDir::new(github_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = match entry.path().strip_prefix(github_root) {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };

        // Skip files belonging to a FIR the user did not select.
        if let Some(fir) = top_level_fir(&rel) {
            if !selected_firs.contains(&fir) {
                continue;
            }
        }

        // Skip duplicate copyright files at non-FIR locations; handled
        // separately for the per-FIR rule below.
        let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if file_name == COPYRIGHT_FILE {
            // Always keep per-FIR copyright if present; the overlay handles it.
        }

        // GNG-owned paths are skipped, with the explicit CoFrance loader exception.
        if is_gng_owned(gng_set, &rel) && !is_cofrance_loader_exception(&rel) {
            continue;
        }

        plan.ops.push(FileOp::Copy {
            src: entry.path().to_path_buf(),
            dst: install_root.join(&rel),
        });
    }
}

fn plan_gng_overlay(
    gng_root: &Path,
    install_root: &Path,
    plan: &mut SyncPlan,
    sector_writes: &mut Vec<FileOp>,
    sector_targets: &mut BTreeSet<(FirCode, SectorExt)>,
) {
    let sectors_dir = install_root.join(SECTORS_SUBPATH);

    for entry in WalkDir::new(gng_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase());

        // Sector files: rename to <FIR>.<ext>, place in LFXX/Sectors/.
        if let Some(ext) = ext.as_deref().and_then(SectorExt::from_str) {
            if let Some((fir, cycle)) = parse_gng_sector_filename(&name) {
                if let Some(c) = cycle.clone() {
                    plan.detected_airac = Some(c);
                }
                let dst = sectors_dir.join(format!("{}.{}", fir.as_str(), ext.as_str()));
                sector_writes.push(FileOp::WriteSector {
                    src: path.to_path_buf(),
                    dst,
                    fir,
                    ext,
                });
                sector_targets.insert((fir, ext));
                continue;
            }
            // Unrecognised .sct/.ese — skip with warning (e.g. LFFM).
            plan.warnings
                .push(format!("Ignoring unrecognised sector file: {}", name));
            continue;
        }

        // `.rwy` files: pair with the sector dir.
        if ext.as_deref() == Some("rwy") {
            if let Some((fir, _)) = parse_gng_sector_filename(&name) {
                let dst = sectors_dir.join(format!("{}.rwy", fir.as_str()));
                plan.ops.push(FileOp::Copy {
                    src: path.to_path_buf(),
                    dst,
                });
                continue;
            }
        }

        // `.prf` files: move into <FIR>/Profiles/.
        if ext.as_deref() == Some("prf") {
            if let Some(fir) = locate_fir_for_prf(gng_root, path) {
                let dst = install_root
                    .join(fir.as_str())
                    .join("Profiles")
                    .join(path.file_name().unwrap());
                plan.ops.push(FileOp::MoveProfile {
                    src: path.to_path_buf(),
                    dst,
                });
                continue;
            }
            plan.warnings.push(format!(
                "Skipping .prf with no recognisable FIR: {}",
                path.display()
            ));
            continue;
        }

        // All other files: only kept if they live under a recognised FIR path
        // inside the GNG archive AND that path is in our GNG-owned list (i.e.
        // ICAO, NavData, Alias, settings the FIR owns).
        if let Some(rel) = locate_inside_fir_or_lfxx(gng_root, path) {
            let gng_set = gng_owned_set();
            // Apply GNG-owned paths (these are exactly the paths GNG should provide).
            if is_gng_owned(&gng_set, &rel) {
                plan.ops.push(FileOp::Copy {
                    src: path.to_path_buf(),
                    dst: install_root.join(&rel),
                });
                continue;
            }
            // Per-FIR copyright file is always preserved.
            if rel
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase() == COPYRIGHT_FILE)
                .unwrap_or(false)
            {
                plan.ops.push(FileOp::Copy {
                    src: path.to_path_buf(),
                    dst: install_root.join(&rel),
                });
            }
        }
    }
}

fn locate_fir_for_prf(gng_root: &Path, path: &Path) -> Option<FirCode> {
    let rel = path.strip_prefix(gng_root).ok()?;
    for part in rel.iter() {
        if let Ok(fir) = part.to_string_lossy().parse::<FirCode>() {
            return Some(fir);
        }
    }
    // If the .prf sits at the GNG archive root, try to infer from the filename.
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    for fir in FirCode::ALL {
        if stem.contains(fir.as_str()) {
            return Some(fir);
        }
    }
    None
}

/// Returns the destination-relative path for a GNG file, anchored at the
/// first FIR/LFXX segment encountered in the package. Returns `None` if no
/// such segment exists.
fn locate_inside_fir_or_lfxx(gng_root: &Path, path: &Path) -> Option<PathBuf> {
    let rel = path.strip_prefix(gng_root).ok()?;
    let parts: Vec<_> = rel.iter().collect();
    for (idx, part) in parts.iter().enumerate() {
        let s = part.to_string_lossy();
        let upper = s.to_ascii_uppercase();
        if upper == "LFXX" || FirCode::ALL.iter().any(|fir| fir.as_str() == upper.as_str()) {
            let tail: PathBuf = parts[idx..].iter().collect();
            return Some(tail);
        }
        if upper == "LFFM" {
            // Legacy: rewrite LFFM into LFXX.
            let mut p = PathBuf::from("LFXX");
            for tail_part in &parts[idx + 1..] {
                p.push(tail_part);
            }
            return Some(p);
        }
    }
    None
}

fn plan_sector_backups(
    install_root: &Path,
    sector_targets: &BTreeSet<(FirCode, SectorExt)>,
    previous_airac: Option<&str>,
    plan: &mut SyncPlan,
) {
    let sectors_dir = install_root.join(SECTORS_SUBPATH);
    let backup_dir = sectors_dir.join(SECTOR_BACKUP_DIRNAME);

    plan.ops.insert(0, FileOp::EnsureDir {
        path: backup_dir.clone(),
    });

    for (fir, ext) in sector_targets {
        let existing = sectors_dir.join(format!("{}.{}", fir.as_str(), ext.as_str()));
        if existing.exists() {
            let cycle = previous_airac.unwrap_or("unknown");
            let mut backup = backup_dir.join(format!("{}-{}.{}", fir.as_str(), cycle, ext.as_str()));
            if backup.exists() {
                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                backup = backup_dir.join(format!(
                    "{}-{}_{}.{}",
                    fir.as_str(),
                    cycle,
                    ts,
                    ext.as_str()
                ));
            }
            plan.ops.push(FileOp::BackupSector {
                src: existing,
                dst: backup,
            });
        }
    }
}

fn read_current_airac(install_root: &Path) -> Option<String> {
    let path = install_root.join(CURRENT_AIRAC_FILE);
    let content = std::fs::read_to_string(&path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// Convert a SyncPlan into a result summary after apply.
pub fn summarize(plan: &SyncPlan, written: usize, skipped: usize) -> SyncSummary {
    SyncSummary {
        github_sha: plan.github_short_sha.clone(),
        airac_cycle: plan.detected_airac.clone(),
        files_written: written,
        files_skipped: skipped,
        warnings: plan.warnings.clone(),
    }
}

