use crate::github;
use crate::profile_store;
use anyhow::Context;
use controller_pack_core::pack_sync::{apply, plan, PlanInputs, SyncSummary};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(serde::Serialize, Clone)]
struct Progress {
    step: String,
}

fn emit(app: &AppHandle, step: &str) {
    let _ = app.emit("sync:progress", Progress { step: step.into() });
}

pub async fn run_sync(
    app: &AppHandle,
    package_paths: Vec<PathBuf>,
    also_apply_profile: Option<bool>,
) -> anyhow::Result<SyncSummary> {
    let profile = profile_store::load(app)?;
    let install_root = profile
        .controller_pack_dir
        .clone()
        .context("controller pack directory is not set")?;

    emit(app, "Resolving latest GitHub revision");
    let github_short_sha = github::get_short_sha().await.ok();

    emit(app, "Downloading GitHub repository");
    let github_repo = github::download_repo()
        .await
        .context("failed to download GitHub repo")?;

    emit(app, "Extracting selected packages");
    let mut packages = Vec::with_capacity(package_paths.len());
    for path in &package_paths {
        packages.push(
            crate::local_packages::extract_package(path)
                .with_context(|| format!("extracting {}", path.display()))?,
        );
    }
    let pack_roots: Vec<PathBuf> = packages.iter().map(|p| p.root().to_path_buf()).collect();

    emit(app, "Planning file operations");
    let plan_result = plan(PlanInputs {
        github_root: Some(github_repo.root()),
        gng_roots: &pack_roots,
        install_root: &install_root,
        github_short_sha: github_short_sha.clone(),
    })?;

    emit(app, "Applying changes");
    let summary = apply(&install_root, &plan_result)?;

    // Diagnostic notes (files intentionally skipped) — debug only, not warnings.
    for note in &plan_result.notes {
        tracing::debug!("{note}");
    }
    // Real warnings: surface each (the summary only carries a count).
    if !summary.warnings.is_empty() {
        tracing::warn!(count = summary.warnings.len(), "sync produced warnings:");
        for w in &summary.warnings {
            tracing::warn!("  • {w}");
        }
    }

    // Update persisted versions.
    let mut updated = profile_store::load(app)?;
    if let Some(sha) = &summary.github_sha {
        updated.versions.installed_github_sha = Some(sha.clone());
    }
    if let Some(cycle) = &summary.airac_cycle {
        updated.versions.installed_airac_cycle = Some(cycle.clone());
    }
    profile_store::save(app, &updated)?;

    let apply_profile = also_apply_profile
        .unwrap_or(profile.preferences.apply_creds_after_sync);
    if apply_profile && !updated.vatsim.cid.is_empty() {
        emit(app, "Applying profile credentials");
        controller_pack_core::profile_configurator::apply(&install_root, &updated)?;
    }

    emit(app, "Done");
    Ok(summary)
}
