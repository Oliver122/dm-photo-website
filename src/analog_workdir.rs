//! Job workdir paths and safe file resolution for analog ingest preview (REQ-007).

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use thiserror::Error;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkdirPathError {
    #[error("path traversal rejected")]
    Traversal,
    #[error("path not found")]
    NotFound,
}

pub fn job_work_dir(ingest_root: &Path, job_id: i64) -> PathBuf {
    ingest_root.join(job_id.to_string())
}

/// Resolve `relative` under `work_dir`. Rejects `..`, absolute paths, and escapes.
pub fn resolve_workdir_file(work_dir: &Path, relative: &str) -> Result<PathBuf, WorkdirPathError> {
    if relative.is_empty() || relative.contains("..") {
        return Err(WorkdirPathError::Traversal);
    }

    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(WorkdirPathError::Traversal);
    }
    for component in rel.components() {
        if matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)) {
            return Err(WorkdirPathError::Traversal);
        }
    }

    let full = work_dir.join(rel);
    let work_dir = work_dir
        .canonicalize()
        .map_err(|_| WorkdirPathError::NotFound)?;
    let full = full
        .canonicalize()
        .map_err(|_| WorkdirPathError::NotFound)?;

    if !full.starts_with(&work_dir) {
        return Err(WorkdirPathError::Traversal);
    }

    Ok(full)
}

fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false)
}

/// List relative paths of image files under `work_dir/images/`.
pub fn list_preview_images(work_dir: &Path) -> Result<Vec<String>> {
    let images_dir = work_dir.join("images");
    if !images_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&images_dir).with_context(|| {
        format!("read preview images dir {}", images_dir.display())
    })? {
        let entry = entry.context("read preview dir entry")?;
        let path = entry.path();
        if path.is_file() && is_image_path(&path) {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .context("non-utf8 file name in preview dir")?;
            paths.push(format!("images/{file_name}"));
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}

pub async fn remove_job_workdir(work_dir: &Path) -> Result<()> {
    match tokio::fs::remove_dir_all(work_dir).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => bail!("failed to remove work dir {}: {err}", work_dir.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_rejects_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_workdir_file(dir.path(), "../etc/passwd"),
            Err(WorkdirPathError::Traversal)
        );
        assert_eq!(
            resolve_workdir_file(dir.path(), "images/../../secret"),
            Err(WorkdirPathError::Traversal)
        );
    }

    #[test]
    fn resolve_accepts_file_under_workdir() {
        let dir = tempfile::tempdir().unwrap();
        let images = dir.path().join("images");
        fs::create_dir_all(&images).unwrap();
        let file = images.join("frame.jpg");
        fs::write(&file, b"fake").unwrap();

        let resolved = resolve_workdir_file(dir.path(), "images/frame.jpg").unwrap();
        assert_eq!(resolved, file.canonicalize().unwrap());
    }
}
