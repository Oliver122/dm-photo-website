use std::path::{Path, PathBuf};

use rand::RngCore;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Client for PhotoPrism `/api/v1` session + upload → import flow.
#[derive(Debug, Clone)]
pub struct PhotoPrismClient {
    base_url: String,
    username: String,
    password: String,
    /// Optional expected UID; session `user.UID` is preferred for upload paths.
    expected_user_uid: Option<String>,
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
    #[error("PhotoPrism session login failed (status {status}): {body}")]
    SessionLogin { status: u16, body: String },
    #[error("PhotoPrism session response missing access_token or user.UID")]
    SessionIncomplete,
    #[error("PhotoPrism stage upload failed (status {status}): {body}")]
    StageUpload { status: u16, body: String },
    #[error("PhotoPrism import commit failed (status {status}): {body}")]
    CommitImport { status: u16, body: String },
    #[error("PhotoPrism album lookup failed (status {status}): {body}")]
    AlbumLookup { status: u16, body: String },
    #[error("PhotoPrism album create failed (status {status}): {body}")]
    AlbumCreate { status: u16, body: String },
    #[error("PhotoPrism album create response missing UID for title {title}")]
    AlbumIncomplete { title: String },
}

#[derive(Debug, Serialize)]
struct SessionLoginBody<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
struct SessionUser {
    #[serde(rename = "UID")]
    uid: String,
}

/// Minimal fields from `POST /api/v1/session` needed for uploads.
#[derive(Debug, Deserialize)]
pub struct SessionResponse {
    access_token: String,
    user: SessionUser,
}

#[derive(Debug, Serialize)]
struct ImportBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    albums: Option<&'a [String]>,
}

#[derive(Debug, Serialize)]
struct CreateAlbumBody<'a> {
    #[serde(rename = "Title")]
    title: &'a str,
    #[serde(rename = "Favorite")]
    favorite: bool,
}

#[derive(Debug, Deserialize)]
struct AlbumSummary {
    #[serde(rename = "UID")]
    uid: String,
    #[serde(rename = "Title")]
    title: String,
}

#[derive(Debug)]
struct ActiveSession {
    access_token: String,
    user_uid: String,
}

impl PhotoPrismClient {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        expected_user_uid: Option<String>,
        verify_tls: bool,
    ) -> Result<Self, PhotoPrismError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .user_agent("dm-photo-website/0.1")
            .danger_accept_invalid_certs(!verify_tls)
            .build()?;

        Ok(Self {
            base_url,
            username: username.into(),
            password: password.into(),
            expected_user_uid,
            http,
        })
    }

    /// Stage `paths` via multipart POST, then PUT to import/index.
    ///
    /// Authenticates with `POST /api/v1/session` and uses the returned
    /// `access_token` as Bearer. When `album_opt` is set, finds or creates
    /// that manual album and passes its UID in the import `albums` list
    /// (titles alone are unreliable on some PhotoPrism builds).
    pub async fn upload_files(
        &self,
        paths: &[PathBuf],
        album_opt: Option<&str>,
    ) -> Result<(), PhotoPrismError> {
        if paths.is_empty() {
            return Err(PhotoPrismError::EmptyPaths);
        }

        let session = self.create_session().await?;
        let token = generate_upload_token();
        let upload_url = format!(
            "{}/api/v1/users/{}/upload/{}",
            self.base_url, session.user_uid, token
        );

        let result = async {
            let album_uid = match album_opt.map(str::trim).filter(|t| !t.is_empty()) {
                Some(title) => Some(self.ensure_album(&session, title).await?),
                None => None,
            };

            for path in paths {
                self.stage_file(&upload_url, path, &session.access_token)
                    .await?;
            }
            self.commit_import(&upload_url, album_uid.as_deref(), &session.access_token)
                .await?;
            Ok(())
        }
        .await;

        self.delete_session(&session.access_token).await;
        result
    }

    /// Return an existing manual album UID for `title`, or create it.
    async fn ensure_album(
        &self,
        session: &ActiveSession,
        title: &str,
    ) -> Result<String, PhotoPrismError> {
        if let Some(uid) = self.find_album_uid(session, title).await? {
            tracing::debug!(album = title, %uid, "using existing PhotoPrism album");
            return Ok(uid);
        }

        let url = format!("{}/api/v1/albums", self.base_url);
        let response = self
            .http
            .post(&url)
            .bearer_auth(&session.access_token)
            .json(&CreateAlbumBody {
                title,
                favorite: false,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotoPrismError::AlbumCreate { status, body });
        }

        let created: AlbumSummary = response.json().await.map_err(PhotoPrismError::Network)?;
        if created.uid.is_empty() {
            return Err(PhotoPrismError::AlbumIncomplete {
                title: title.to_string(),
            });
        }

        tracing::info!(album = title, uid = %created.uid, "created PhotoPrism album");
        Ok(created.uid)
    }

    async fn find_album_uid(
        &self,
        session: &ActiveSession,
        title: &str,
    ) -> Result<Option<String>, PhotoPrismError> {
        let url = format!("{}/api/v1/albums", self.base_url);
        let response = self
            .http
            .get(&url)
            .bearer_auth(&session.access_token)
            .query(&[("count", "100"), ("type", "album"), ("q", title)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotoPrismError::AlbumLookup { status, body });
        }

        let albums: Vec<AlbumSummary> = response.json().await.map_err(PhotoPrismError::Network)?;
        let exact = albums
            .into_iter()
            .find(|album| album.title.eq_ignore_ascii_case(title));
        Ok(exact.map(|album| album.uid))
    }

    async fn create_session(&self) -> Result<ActiveSession, PhotoPrismError> {
        let url = format!("{}/api/v1/session", self.base_url);
        let response = self
            .http
            .post(&url)
            .json(&SessionLoginBody {
                username: &self.username,
                password: &self.password,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(PhotoPrismError::SessionLogin { status, body });
        }

        let parsed: SessionResponse = response.json().await.map_err(PhotoPrismError::Network)?;
        let session = active_session_from_response(parsed, self.expected_user_uid.as_deref())?;
        Ok(session)
    }

    async fn delete_session(&self, access_token: &str) {
        let url = format!("{}/api/v1/session", self.base_url);
        if let Err(err) = self
            .http
            .delete(&url)
            .bearer_auth(access_token)
            .send()
            .await
        {
            tracing::debug!(error = %err, "PhotoPrism session logout failed (ignored)");
        }
    }

    async fn stage_file(
        &self,
        upload_url: &str,
        path: &Path,
        access_token: &str,
    ) -> Result<(), PhotoPrismError> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|source| PhotoPrismError::ReadFile {
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
            .bearer_auth(access_token)
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
        album_uid_opt: Option<&str>,
        access_token: &str,
    ) -> Result<(), PhotoPrismError> {
        let albums = album_uid_opt.map(|uid| vec![uid.to_string()]);
        let body = ImportBody {
            albums: albums.as_deref(),
        };

        let response = self
            .http
            .put(upload_url)
            .bearer_auth(access_token)
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

fn active_session_from_response(
    parsed: SessionResponse,
    expected_user_uid: Option<&str>,
) -> Result<ActiveSession, PhotoPrismError> {
    if parsed.access_token.is_empty() || parsed.user.uid.is_empty() {
        return Err(PhotoPrismError::SessionIncomplete);
    }

    if let Some(expected) = expected_user_uid {
        if expected != parsed.user.uid {
            tracing::warn!(
                expected,
                got = %parsed.user.uid,
                "PHOTOPRISM_USER_UID does not match session user.UID; using session UID"
            );
        }
    }

    Ok(ActiveSession {
        access_token: parsed.access_token,
        user_uid: parsed.user.uid,
    })
}

/// Client-generated upload batch token (PhotoPrism UI: 7× `[a-z0-9]`).
pub fn generate_upload_token() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut bytes = [0u8; 7];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_token_matches_photoprism_shape() {
        let token = generate_upload_token();
        assert_eq!(token.len(), 7);
        assert!(
            token
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }

    #[test]
    fn upload_tokens_differ() {
        let a = generate_upload_token();
        let b = generate_upload_token();
        assert_ne!(a, b);
    }

    #[test]
    fn parse_session_response_extracts_token_and_uid() {
        let json = r#"{
            "access_token": "tok-abc",
            "status": "success",
            "user": {"ID": 1, "UID": "utexva5sbdnpvgc0", "Name": "admin"}
        }"#;
        let parsed: SessionResponse = serde_json::from_str(json).expect("parse");
        let session = active_session_from_response(parsed, Some("other")).expect("session");
        assert_eq!(session.access_token, "tok-abc");
        assert_eq!(session.user_uid, "utexva5sbdnpvgc0");
    }

    #[test]
    fn parse_session_rejects_empty_token() {
        let json = r#"{"access_token":"","user":{"UID":"u1"}}"#;
        let parsed: SessionResponse = serde_json::from_str(json).expect("parse");
        let err = active_session_from_response(parsed, None).unwrap_err();
        assert!(matches!(err, PhotoPrismError::SessionIncomplete));
    }

    #[test]
    fn create_album_body_uses_photoprism_field_names() {
        let body = CreateAlbumBody {
            title: "Analog 2026",
            favorite: false,
        };
        let json = serde_json::to_string(&body).expect("serialize");
        assert_eq!(json, r#"{"Title":"Analog 2026","Favorite":false}"#);
    }

    #[test]
    fn parse_album_summary() {
        let json = r#"{"UID":"atest123","Title":"test","Slug":"test","PhotoCount":3}"#;
        let album: AlbumSummary = serde_json::from_str(json).expect("parse");
        assert_eq!(album.uid, "atest123");
        assert_eq!(album.title, "test");
    }

    #[test]
    fn import_body_serializes_album_uid() {
        let albums = vec!["atest123".to_string()];
        let body = ImportBody {
            albums: Some(albums.as_slice()),
        };
        let json = serde_json::to_string(&body).expect("serialize");
        assert_eq!(json, r#"{"albums":["atest123"]}"#);
    }
}
