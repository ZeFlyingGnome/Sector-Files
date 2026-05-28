use super::client::{authed_client, classify_response, GngAuthError, GNG_BASE_URL};
use controller_pack_core::FirCode;
use crate::profile_store::GngSession;
use anyhow::Context;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tempfile::TempDir;

/// One extracted GNG package, kept alive (via its tempdir) for the lifetime of
/// the sync run.
pub struct GngPackage {
    _tmp: TempDir,
    root: PathBuf,
    pub fir: Option<FirCode>,
}

impl GngPackage {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Download the FIR install/update packages plus the LFXX CoFrance base from
/// GNG. Returns one `GngPackage` per archive successfully extracted.
///
/// TODO (task 5.4): the GNG download URL scheme is not yet known. Two paths
/// are sketched in `gng/README.md`; until either is implemented, this returns
/// an empty list and `run_sync` proceeds with the GitHub overlay only.
pub async fn download_packages(
    _app: &AppHandle,
    session: &GngSession,
    selected_firs: &[FirCode],
) -> anyhow::Result<Vec<GngPackage>> {
    if session.cookies.is_empty() {
        // No GNG session — caller can proceed with GitHub-only sync.
        return Ok(vec![]);
    }

    let _client = authed_client(session)?;
    let _selected = selected_firs;

    // Once URL discovery is done, this becomes:
    //   for fir in selected_firs.iter().copied().chain([base_lfxx]) {
    //       let url = url_for(fir);
    //       let bytes = client.get(url).send().await?.bytes().await?;
    //       result.push(extract(bytes, fir)?);
    //   }
    Ok(vec![])
}

#[allow(dead_code)]
async fn fetch_archive(
    client: &reqwest::Client,
    url: &str,
) -> Result<bytes::Bytes, GngAuthError> {
    let resp = client.get(url).send().await?;
    classify_response(&resp)?;
    let resp = resp.error_for_status()?;
    Ok(resp.bytes().await?)
}

#[allow(dead_code)]
fn extract_zip(bytes: bytes::Bytes, fir: Option<FirCode>) -> anyhow::Result<GngPackage> {
    let tmp = TempDir::new().context("tempdir for GNG archive")?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec()))?;
    archive.extract(tmp.path())?;
    let root = tmp.path().to_path_buf();
    Ok(GngPackage {
        _tmp: tmp,
        root,
        fir,
    })
}

#[allow(dead_code)]
fn extract_7z(bytes: bytes::Bytes, fir: Option<FirCode>) -> anyhow::Result<GngPackage> {
    let tmp = TempDir::new().context("tempdir for GNG archive")?;
    sevenz_rust::decompress(Cursor::new(bytes.to_vec()), tmp.path())?;
    let root = tmp.path().to_path_buf();
    Ok(GngPackage {
        _tmp: tmp,
        root,
        fir,
    })
}

#[allow(dead_code)]
fn base_url() -> &'static str {
    GNG_BASE_URL
}
