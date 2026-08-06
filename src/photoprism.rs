//! PhotoPrism REST client (stub — replaced by sibling branch on merge).

use std::path::Path;

use thiserror::Error;

use crate::config::PhotoPrismConfig;

#[derive(Debug, Error)]
pub enum PhotoPrismError {
    #[error("PhotoPrism client not implemented yet (todo: merge photoprism module)")]
    NotImplemented,
    #[error("PhotoPrism is not configured")]
    NotConfigured,
    #[error("{0}")]
    Other(String),
}

pub async fn upload_and_import(
    _http: &reqwest::Client,
    config: &PhotoPrismConfig,
    _files: &[std::path::PathBuf],
    _album: Option<&str>,
) -> Result<(), PhotoPrismError> {
    if !config.is_configured() {
        return Err(PhotoPrismError::NotConfigured);
    }
    Err(PhotoPrismError::NotImplemented)
}

pub fn collect_image_paths(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            if matches!(ext.as_deref(), Some("jpg") | Some("jpeg") | Some("tif") | Some("tiff")) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}
