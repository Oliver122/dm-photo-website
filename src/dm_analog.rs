use std::fs::File;
use std::io::{copy, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

const API_BASE: &str = "https://api.cewe-myphotos.com/api/imageCD";
const API_ACCESS_KEY: &str = "54a614716eb29ef3a3f004a6241e5e19";
const CLIENT_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub lab_id: String,
    pub order_ts: i64,
    pub deleted_at_ts: i64,
}

#[derive(Debug, Error)]
pub enum DmAnalogError {
    #[error("Bestellnummer ungültig — Format: 123456-123456")]
    InvalidOrderId,
    #[error("Secure-ID ungültig — genau 8 Zeichen (Buchstaben und Ziffern)")]
    InvalidSecureId,
    #[error("Bestellnummer oder Secure-ID ist falsch")]
    BadCredentials,
    #[error("Download-Zeitraum abgelaufen — Fotos sind nicht mehr verfügbar")]
    Expired,
    #[error("Netzwerkfehler beim CEWE-Download: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Dateifehler: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP konnte nicht gelesen werden: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("ZIP enthält keine Bilder")]
    EmptyZip,
    #[error("CEWE-API-Fehler (HTTP {status}): {detail}")]
    Api { status: u16, detail: String },
}

#[derive(Debug, Deserialize)]
struct MetadataResponse {
    #[serde(rename = "labId")]
    lab_id: String,
    #[serde(rename = "orderTs")]
    order_ts: i64,
    #[serde(rename = "deletedAtTs")]
    deleted_at_ts: i64,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: Option<i32>,
    message: Option<String>,
}

/// Order number must match `^\d{6}-\d{6}$`.
pub fn validate_order_id(order: &str) -> Result<(), DmAnalogError> {
    let bytes = order.as_bytes();
    if bytes.len() != 13
        || bytes[6] != b'-'
        || !bytes[..6].iter().all(u8::is_ascii_digit)
        || !bytes[7..].iter().all(u8::is_ascii_digit)
    {
        return Err(DmAnalogError::InvalidOrderId);
    }
    Ok(())
}

/// Secure-ID must be exactly 8 ASCII alphanumeric characters (case-sensitive).
pub fn validate_secure_id(secure: &str) -> Result<(), DmAnalogError> {
    if secure.len() != 8 || !secure.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(DmAnalogError::InvalidSecureId);
    }
    Ok(())
}

fn validate_credentials(order: &str, secure: &str) -> Result<(), DmAnalogError> {
    validate_order_id(order)?;
    validate_secure_id(secure)?;
    Ok(())
}

fn metadata_url(order: &str, secure: &str) -> String {
    format!("{API_BASE}/{order}/{secure}")
}

fn download_url(order: &str, secure: &str) -> String {
    format!("{API_BASE}/{order}/{secure}/download")
}

fn api_get(http: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    http.get(url)
        .header("apiAccessKey", API_ACCESS_KEY)
        .header("clientVersion", CLIENT_VERSION)
}

fn map_api_error(status: u16, body: &str) -> DmAnalogError {
    let parsed = serde_json::from_str::<ApiErrorBody>(body).ok();
    let code = parsed.as_ref().and_then(|e| e.code);
    let message = parsed
        .and_then(|e| e.message)
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| body.to_string());

    match (status, code) {
        (404, Some(2)) | (404, None) => DmAnalogError::BadCredentials,
        (400, Some(151)) => DmAnalogError::InvalidOrderId,
        (401, Some(3)) => DmAnalogError::Api {
            status,
            detail: "API-Zugangsschlüssel abgelehnt — Konfiguration prüfen".into(),
        },
        _ => DmAnalogError::Api {
            status,
            detail: message,
        },
    }
}

fn check_expiry(deleted_at_ts: i64) -> Result<(), DmAnalogError> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    if now_ms > deleted_at_ts {
        return Err(DmAnalogError::Expired);
    }
    Ok(())
}

/// Fetch order metadata from the CEWE imageCD API.
pub async fn fetch_metadata(
    http: &reqwest::Client,
    order: &str,
    secure: &str,
) -> Result<Metadata, DmAnalogError> {
    validate_credentials(order, secure)?;

    let response = api_get(http, &metadata_url(order, secure))
        .send()
        .await?;
    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_api_error(status, &body));
    }

    let parsed: MetadataResponse = response.json().await?;
    check_expiry(parsed.deleted_at_ts)?;

    Ok(Metadata {
        lab_id: parsed.lab_id,
        order_ts: parsed.order_ts,
        deleted_at_ts: parsed.deleted_at_ts,
    })
}

/// Download the analog photo ZIP to `dest_path`.
pub async fn download_zip(
    http: &reqwest::Client,
    order: &str,
    secure: &str,
    dest_path: &Path,
) -> Result<(), DmAnalogError> {
    validate_credentials(order, secure)?;

    let response = api_get(http, &download_url(order, secure))
        .send()
        .await?;
    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(map_api_error(status, &body));
    }

    let mut file = tokio::fs::File::create(dest_path).await?;
    let mut response = response;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    Ok(())
}

pub fn is_image_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".tif")
        || lower.ends_with(".tiff")
        || lower.ends_with(".png")
}

/// Extract image files from `zip_path` into `dest_dir`. Returns absolute paths.
pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>, DmAnalogError> {
    std::fs::create_dir_all(dest_dir)?;
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let mut images = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if entry.is_dir() || name.contains("..") || !is_image_file_name(&name) {
            continue;
        }
        let file_name = Path::new(&name)
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("image-{i}.jpg")));
        let out_path = dest_dir.join(file_name);
        let mut out = File::create(&out_path)?;
        copy(&mut entry, &mut out)?;
        images.push(out_path);
    }

    if images.is_empty() {
        return Err(DmAnalogError::EmptyZip);
    }
    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_order_id_accepts_dm_format() {
        assert!(validate_order_id("544850-103396").is_ok());
        assert!(validate_order_id("123456-123456").is_ok());
    }

    #[test]
    fn validate_order_id_rejects_bad_formats() {
        for bad in [
            "",
            "544850103396",
            "54485-103396",
            "5448500-103396",
            "544850-10339",
            "abcdef-103396",
            "544850_103396",
            "544850-103396-extra",
        ] {
            assert!(
                matches!(validate_order_id(bad), Err(DmAnalogError::InvalidOrderId)),
                "expected reject for {bad:?}"
            );
        }
    }

    #[test]
    fn validate_secure_id_accepts_alphanumeric_eight_chars() {
        assert!(validate_secure_id("H5GGX3T5").is_ok());
        assert!(validate_secure_id("a1b2c3d4").is_ok());
        assert!(validate_secure_id("ABCD1234").is_ok());
    }

    #[test]
    fn validate_secure_id_is_case_sensitive_length_eight() {
        assert!(validate_secure_id("h5ggx3t5").is_ok(), "lowercase is valid alphanumeric");
        assert!(matches!(
            validate_secure_id("H5GGX3T"),
            Err(DmAnalogError::InvalidSecureId)
        ));
        assert!(matches!(
            validate_secure_id("H5GGX3T55"),
            Err(DmAnalogError::InvalidSecureId)
        ));
        assert!(matches!(
            validate_secure_id("H5GG-T55"),
            Err(DmAnalogError::InvalidSecureId)
        ));
        assert!(matches!(
            validate_secure_id("H5GG 3T5"),
            Err(DmAnalogError::InvalidSecureId)
        ));
    }

    fn write_test_zip(path: &Path, entries: &[(&str, &[u8])]) {
        use std::io::Write;
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, bytes) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extract_zip_pulls_images_skips_others() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("pack.zip");
        write_test_zip(
            &zip_path,
            &[
                ("photos/a.jpg", b"jpeg-a"),
                ("readme.txt", b"nope"),
                ("photos/b.PNG", b"png-b"),
            ],
        );
        let out = dir.path().join("out");
        let images = extract_zip(&zip_path, &out).unwrap();
        assert_eq!(images.len(), 2);
        assert!(images.iter().any(|p| p.file_name().unwrap() == "a.jpg"));
        assert!(images.iter().any(|p| p.file_name().unwrap() == "b.PNG"));
    }

    #[test]
    fn extract_zip_rejects_empty_image_set() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("empty.zip");
        write_test_zip(&zip_path, &[("notes.txt", b"hi")]);
        let err = extract_zip(&zip_path, &dir.path().join("out")).unwrap_err();
        assert!(matches!(err, DmAnalogError::EmptyZip));
    }

    #[test]
    fn extract_zip_skips_path_traversal_names() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        write_test_zip(
            &zip_path,
            &[("../escape.jpg", b"bad"), ("ok.jpeg", b"good")],
        );
        let images = extract_zip(&zip_path, &dir.path().join("out")).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].file_name().unwrap(), "ok.jpeg");
    }
}
