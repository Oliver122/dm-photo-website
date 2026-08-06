use std::path::Path;

use little_exif::{exif_tag::ExifTag, metadata::Metadata};
use thiserror::Error;

const DEFAULT_MAKE: &str = "Analog";

/// Parsed EXIF Make/Model pair from a user-supplied camera label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMakeModel {
    pub make: String,
    pub model: String,
}

#[derive(Debug, Error)]
pub enum CameraExifError {
    #[error("camera label must not be empty")]
    EmptyLabel,
    #[error("failed to read EXIF from {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write EXIF to {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to touch mtime on {path}: {source}")]
    TouchMtime {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Split a user camera label into EXIF Make/Model.
///
/// - `Canon AE-1` → Make=`Canon`, Model=`AE-1` (first whitespace)
/// - `Holga` → Make=`Analog`, Model=`Holga`
pub fn label_to_make_model(label: &str) -> Result<CameraMakeModel, CameraExifError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(CameraExifError::EmptyLabel);
    }

    match trimmed.split_once(char::is_whitespace) {
        Some((make, model)) => {
            let model = model.trim();
            if model.is_empty() {
                Ok(CameraMakeModel {
                    make: DEFAULT_MAKE.into(),
                    model: make.to_string(),
                })
            } else {
                Ok(CameraMakeModel {
                    make: make.to_string(),
                    model: model.to_string(),
                })
            }
        }
        None => Ok(CameraMakeModel {
            make: DEFAULT_MAKE.into(),
            model: trimmed.to_string(),
        }),
    }
}

/// Overwrite IFD0 EXIF `Make` and `Model` on `path` from `label`, preserving other tags.
pub fn stamp_camera_label(path: &Path, label: &str) -> Result<(), CameraExifError> {
    let CameraMakeModel { make, model } = label_to_make_model(label)?;

    let mut meta = Metadata::new_from_path(path).map_err(|source| CameraExifError::Read {
        path: path.display().to_string(),
        source,
    })?;

    meta.set_tag(ExifTag::Make(make));
    meta.set_tag(ExifTag::Model(model));

    meta.write_to_file(path).map_err(|source| CameraExifError::Write {
        path: path.display().to_string(),
        source,
    })?;

    touch_mtime(path)?;

    Ok(())
}

fn touch_mtime(path: &Path) -> Result<(), CameraExifError> {
    let now = filetime::FileTime::now();
    filetime::set_file_mtime(path, now).map_err(|source| CameraExifError::TouchMtime {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_on_first_space() {
        let got = label_to_make_model("Canon AE-1").unwrap();
        assert_eq!(
            got,
            CameraMakeModel {
                make: "Canon".into(),
                model: "AE-1".into(),
            }
        );
    }

    #[test]
    fn split_leica_m6() {
        let got = label_to_make_model("Leica M6").unwrap();
        assert_eq!(
            got,
            CameraMakeModel {
                make: "Leica".into(),
                model: "M6".into(),
            }
        );
    }

    #[test]
    fn single_token_defaults_make_to_analog() {
        let got = label_to_make_model("Holga").unwrap();
        assert_eq!(
            got,
            CameraMakeModel {
                make: "Analog".into(),
                model: "Holga".into(),
            }
        );
    }

    #[test]
    fn multi_word_model_after_first_split() {
        let got = label_to_make_model("My custom rig").unwrap();
        assert_eq!(
            got,
            CameraMakeModel {
                make: "My".into(),
                model: "custom rig".into(),
            }
        );
    }

    #[test]
    fn trims_outer_whitespace() {
        let got = label_to_make_model("  Nikon FM2  ").unwrap();
        assert_eq!(
            got,
            CameraMakeModel {
                make: "Nikon".into(),
                model: "FM2".into(),
            }
        );
    }

    #[test]
    fn rejects_empty_label() {
        assert!(matches!(
            label_to_make_model("   "),
            Err(CameraExifError::EmptyLabel)
        ));
    }
}
