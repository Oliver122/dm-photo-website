use std::path::{Path, PathBuf};

use rand::RngCore;
use reqwest::multipart::{Form, Part};
use serde::Serialize;
use thiserror::Error;

/// Client for PhotoPrism `/api/v1` upload → import flow.
#[derive(Debug, Clone)]
pub struct PhotoPrismClient {
    base_url: String,
    app_password: String,
    user_uid: String,
    http: reqwest::Client,
}

#[derive(Debug, Error)]
pub enum PhotoPrismError {
    #[error("no files to upload")]
    EmptyPaths,
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("network error talking to PhotoPrism: {0}")]
    Network(#[from] reqwest::Error),
    #[error("PhotoPrism stage upload failed (status {status}): {body}")]
    StageUpload { status: u16, body: String },
    #[error("PhotoPrism import commit failed (status {status}): {body}")]
    CommitImport { status: u16, body: String },
}

#[derive(Debug, Serialize)]
struct ImportBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    albums: Option<&'a [String]>,
}

impl PhotoPrismClient {
    pub fn new(
        base_url: impl Into<String>,
        app_password: impl Into<String>,
        user_uid: impl Into<String>,
        verify_tls: bool,
    ) -> Result<Self, PhotoPrismError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .user_agent("dm-photo-website/0.1")
            .danger_accept_invalid_certs(!verify_tls)
            .build()?;

        Ok(Self {
            base_url,
            app_password: app_password.into(),
            user_uid: user_uid.into(),
            http,
        })
    }

    /// Stage `paths` via multipart POST, then PUT to import/index.
    ///
    /// `album_opt` is passed as the optional `albums` array (UID or title).
    pub async fn upload_files(
        &self,
        paths: &[PathBuf],
        album_opt: Option<&str>,
    ) -> Result<(), PhotoPrismError> {
        if paths.is_empty() {
            return Err(PhotoPrismError::EmptyPaths);
        }

        let token = generate_upload_token();
        let upload_url = format!(
            "{}/api/v1/users/{}/upload/{}",
            self.base_url, self.user_uid, token
        );

        for path in paths {
            self.stage_file(&upload_url, path).await?;
        }

        self.commit_import(&upload_url, album_opt).await?;

        Ok(())
    }

    async fn stage_file(&self, upload_url: &str, path: &Path) -> Result<(), PhotoPrismError> {
        let bytes = tokio::fs::read(path).await.map_err(|source| PhotoPrismError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload.jpg")
            .to_string();

        let part = Part::bytes(bytes)
            .file_name(file_name)
            .mime_str("application/octet-stream")
            .map_err(PhotoPrismError::Network)?;

        let form = Form::new().part("files", part);

        let response = self
            .http
            .post(upload_url)
            .bearer_auth(&self.app_password)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotoPrismError::StageUpload { status, body });
        }

        Ok(())
    }

    async fn commit_import(
        &self,
        upload_url: &str,
        album_opt: Option<&str>,
    ) -> Result<(), PhotoPrismError> {
        let albums = album_opt.map(|title| vec![title.to_string()]);
        let body = ImportBody {
            albums: albums.as_deref(),
        };

        let response = self
            .http
            .put(upload_url)
            .bearer_auth(&self.app_password)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotoPrismError::CommitImport { status, body });
        }

        Ok(())
    }
}

/// Client-generated upload session token (random hex).
pub fn generate_upload_token() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_token_is_hex() {
        let token = generate_upload_token();
        assert_eq!(token.len(), 16);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn upload_tokens_differ() {
        let a = generate_upload_token();
        let b = generate_upload_token();
        assert_ne!(a, b);
    }
}
