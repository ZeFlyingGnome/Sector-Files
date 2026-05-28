use anyhow::Context;
use serde::Deserialize;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub const OWNER: &str = "vaccfr";
pub const REPO: &str = "Sector-Files";
pub const BRANCH: &str = "main";

#[derive(Debug, Deserialize)]
struct CommitResponse {
    sha: String,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(crate::gng::client::user_agent())
        .build()
        .expect("reqwest client")
}

pub async fn get_short_sha() -> anyhow::Result<String> {
    let url = format!("https://api.github.com/repos/{OWNER}/{REPO}/commits/{BRANCH}");
    let resp = client().get(&url).send().await?.error_for_status()?;
    let body: CommitResponse = resp.json().await?;
    Ok(body.sha.chars().take(7).collect())
}

/// Download the GitHub repo zipball into a tempdir and return the path to the
/// extracted top-level directory.
pub async fn download_repo() -> anyhow::Result<DownloadedRepo> {
    let url = format!("https://api.github.com/repos/{OWNER}/{REPO}/zipball/{BRANCH}");
    let bytes = client()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let tmp = TempDir::new().context("create tempdir for github zipball")?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    archive.extract(tmp.path())?;

    // GitHub zipballs contain a single top-level directory like `vaccfr-Sector-Files-<sha>`.
    let root = std::fs::read_dir(tmp.path())?
        .filter_map(Result::ok)
        .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .context("github zipball did not contain a top-level directory")?;

    Ok(DownloadedRepo { _tmp: tmp, root })
}

pub struct DownloadedRepo {
    _tmp: TempDir,
    root: PathBuf,
}

impl DownloadedRepo {
    pub fn root(&self) -> &Path {
        &self.root
    }
}
