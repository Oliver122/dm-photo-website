//! dm/CEWE analog download client (stub — replaced by sibling branch on merge).

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DmAnalogCredentials {
    pub order_number: String,
    pub secure_id: String,
}

#[derive(Debug, Error)]
pub enum DmAnalogError {
    #[error("dm analog download not implemented yet (todo: merge dm_analog client)")]
    NotImplemented,
    #[error("invalid order number format")]
    InvalidOrderNumber,
    #[error("invalid Secure-ID format")]
    InvalidSecureId,
    #[error("{0}")]
    Other(String),
}

pub fn validate_order_number(order_number: &str) -> bool {
    let bytes = order_number.as_bytes();
    bytes.len() == 13
        && bytes[6] == b'-'
        && bytes[..6].iter().all(|b| b.is_ascii_digit())
        && bytes[7..].iter().all(|b| b.is_ascii_digit())
}

pub fn validate_secure_id(secure_id: &str) -> bool {
    secure_id.len() == 8
        && secure_id
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

pub async fn download_zip(
    _http: &reqwest::Client,
    _creds: &DmAnalogCredentials,
    _dest: &Path,
) -> Result<(), DmAnalogError> {
    Err(DmAnalogError::NotImplemented)
}

pub fn extract_zip(_zip_path: &Path, _dest_dir: &Path) -> Result<Vec<PathBuf>, DmAnalogError> {
    Err(DmAnalogError::NotImplemented)
}
