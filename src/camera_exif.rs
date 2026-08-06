//! Camera EXIF stamping (stub — replaced by sibling branch on merge).

use std::path::Path;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CameraLabel {
    pub make: String,
    pub model: String,
}

#[derive(Debug, Error)]
pub enum LabelError {
    #[error("camera label must not be empty")]
    Empty,
    #[error("camera label must be at most 64 characters")]
    TooLong,
}

#[derive(Debug, Error)]
pub enum StampError {
    #[error("camera EXIF stamping not implemented yet (todo: merge camera_exif module)")]
    NotImplemented,
    #[error("{0}")]
    Other(String),
}

impl CameraLabel {
    pub fn from_user_label(label: &str) -> Result<Self, LabelError> {
        let trimmed = label.trim();
        if trimmed.is_empty() {
            return Err(LabelError::Empty);
        }
        if trimmed.len() > 64 {
            return Err(LabelError::TooLong);
        }

        match trimmed.split_once(' ') {
            Some((make, model)) if !model.is_empty() => Ok(Self {
                make: make.to_string(),
                model: model.to_string(),
            }),
            _ => Ok(Self {
                make: "Analog".to_string(),
                model: trimmed.to_string(),
            }),
        }
    }
}

pub fn stamp_camera_metadata(_path: &Path, _camera: &CameraLabel) -> Result<(), StampError> {
    Err(StampError::NotImplemented)
}
