use std::path::Path;

use little_exif::{exif_tag::ExifTag, metadata::Metadata, rational::uR64};
use thiserror::Error;

const DEFAULT_MAKE: &str = "Analog";

/// Inclusive lower bound for film ISO stamping.
pub const FILM_ISO_MIN: u32 = 1;
/// Inclusive upper bound for film ISO stamping.
pub const FILM_ISO_MAX: u32 = 102400;

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
    #[error("film ISO must be between {FILM_ISO_MIN} and {FILM_ISO_MAX}, got {value}")]
    InvalidFilmIso { value: u32 },
    #[error("focal length must be greater than 0, got {value}")]
    InvalidFocalMm { value: f64 },
    #[error("aperture must be greater than 0, got {value}")]
    InvalidAperture { value: f64 },
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

/// Validate film ISO for ingest stamping (`1..=102400`).
pub fn validate_film_iso(iso: u32) -> Result<(), CameraExifError> {
    if iso < FILM_ISO_MIN || iso > FILM_ISO_MAX {
        return Err(CameraExifError::InvalidFilmIso { value: iso });
    }
    Ok(())
}

/// Validate lens focal length in millimeters (`> 0`).
pub fn validate_focal_mm(focal_mm: f64) -> Result<(), CameraExifError> {
    if !focal_mm.is_finite() || focal_mm <= 0.0 {
        return Err(CameraExifError::InvalidFocalMm { value: focal_mm });
    }
    Ok(())
}

/// Validate lens aperture / f-number (`> 0`).
pub fn validate_aperture(aperture: f64) -> Result<(), CameraExifError> {
    if !aperture.is_finite() || aperture <= 0.0 {
        return Err(CameraExifError::InvalidAperture { value: aperture });
    }
    Ok(())
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
    stamp_ingest_metadata(path, label, None, None, None)
}

/// Stamp camera Make/Model plus optional film ISO, focal length, and aperture on `path`.
///
/// When set, ISO is written as `ISOSpeedRatings` (`ExifTag::ISO`) and `ISOSpeed`
/// (PhotographicSensitivity). Lens values use `FocalLength` and `FNumber`.
pub fn stamp_ingest_metadata(
    path: &Path,
    camera_label: &str,
    iso: Option<u32>,
    focal_mm: Option<f64>,
    aperture: Option<f64>,
) -> Result<(), CameraExifError> {
    let CameraMakeModel { make, model } = label_to_make_model(camera_label)?;

    if let Some(iso) = iso {
        validate_film_iso(iso)?;
    }
    if let Some(focal_mm) = focal_mm {
        validate_focal_mm(focal_mm)?;
    }
    if let Some(aperture) = aperture {
        validate_aperture(aperture)?;
    }

    let mut meta = metadata_from_path(path)?;

    meta.set_tag(ExifTag::Make(make));
    meta.set_tag(ExifTag::Model(model));

    if let Some(iso) = iso {
        let iso_u16 = iso as u16;
        meta.set_tag(ExifTag::ISO(vec![iso_u16]));
        meta.set_tag(ExifTag::ISOSpeed(vec![iso]));
    }
    if let Some(focal_mm) = focal_mm {
        meta.set_tag(ExifTag::FocalLength(vec![uR64::from(focal_mm)]));
    }
    if let Some(aperture) = aperture {
        meta.set_tag(ExifTag::FNumber(vec![uR64::from(aperture)]));
    }

    meta.write_to_file(path)
        .map_err(|source| CameraExifError::Write {
            path: path.display().to_string(),
            source,
        })?;

    touch_mtime(path)?;

    Ok(())
}

fn metadata_from_path(path: &Path) -> Result<Metadata, CameraExifError> {
    match Metadata::new_from_path(path) {
        Ok(meta) => Ok(meta),
        Err(source) => {
            // Always start fresh when read fails. CEWE scans and post-rotate
            // JPEG rewrites often have no APP1; little_exif then returns
            // Err("No EXIF data found!") instead of empty Metadata. We overwrite
            // the tags we care about anyway, so preserving other tags is best-effort.
            tracing::debug!(
                path = %path.display(),
                error = %source,
                "EXIF read failed; creating new metadata for stamp"
            );
            Ok(Metadata::new())
        }
    }
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
    use std::fs::File;
    use std::io::{BufWriter, Write};

    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageBuffer, Rgb, RgbImage};

    use super::*;

    fn write_fixture_jpeg(path: &Path) {
        let mut img: RgbImage = ImageBuffer::new(8, 6);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([
                (x as u8) * 25 + 10,
                (y as u8) * 35 + 15,
                ((x + y) as u8) * 20 + 5,
            ]);
        }
        let file = File::create(path).expect("create fixture jpeg");
        let mut writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(&mut writer, 90);
        encoder
            .encode(
                img.as_raw(),
                img.width(),
                img.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("encode fixture jpeg");
        writer.flush().expect("flush fixture jpeg");
    }

    fn first_iso(meta: &Metadata) -> Option<u16> {
        for tag in meta.get_tag(&ExifTag::ISO(vec![0])) {
            if let ExifTag::ISO(vals) = tag {
                return vals.first().copied();
            }
        }
        None
    }

    fn first_iso_speed(meta: &Metadata) -> Option<u32> {
        for tag in meta.get_tag(&ExifTag::ISOSpeed(vec![0])) {
            if let ExifTag::ISOSpeed(vals) = tag {
                return vals.first().copied();
            }
        }
        None
    }

    fn first_focal_length(meta: &Metadata) -> Option<f64> {
        for tag in meta.get_tag(&ExifTag::FocalLength(vec![uR64::from(0.0)])) {
            if let ExifTag::FocalLength(vals) = tag {
                return vals.first().map(|r| f64::from(r.clone()));
            }
        }
        None
    }

    fn first_f_number(meta: &Metadata) -> Option<f64> {
        for tag in meta.get_tag(&ExifTag::FNumber(vec![uR64::from(0.0)])) {
            if let ExifTag::FNumber(vals) = tag {
                return vals.first().map(|r| f64::from(r.clone()));
            }
        }
        None
    }

    fn first_make(meta: &Metadata) -> Option<String> {
        for tag in meta.get_tag(&ExifTag::Make(String::new())) {
            if let ExifTag::Make(make) = tag {
                return Some(make.clone());
            }
        }
        None
    }

    fn first_model(meta: &Metadata) -> Option<String> {
        for tag in meta.get_tag(&ExifTag::Model(String::new())) {
            if let ExifTag::Model(model) = tag {
                return Some(model.clone());
            }
        }
        None
    }

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

    /// T-006-a: film ISO, focal length, and aperture validation accept/reject.
    #[test]
    fn t_006_a_iso_focal_aperture_validation() {
        assert!(validate_film_iso(1).is_ok());
        assert!(validate_film_iso(400).is_ok());
        assert!(validate_film_iso(FILM_ISO_MAX).is_ok());
        assert!(matches!(
            validate_film_iso(0),
            Err(CameraExifError::InvalidFilmIso { value: 0 })
        ));
        assert!(matches!(
            validate_film_iso(102401),
            Err(CameraExifError::InvalidFilmIso { value: 102401 })
        ));

        assert!(validate_focal_mm(50.0).is_ok());
        assert!(validate_focal_mm(0.5).is_ok());
        assert!(matches!(
            validate_focal_mm(0.0),
            Err(CameraExifError::InvalidFocalMm { value: 0.0 })
        ));
        assert!(matches!(
            validate_focal_mm(-1.0),
            Err(CameraExifError::InvalidFocalMm { value: -1.0 })
        ));

        assert!(validate_aperture(2.4).is_ok());
        assert!(validate_aperture(1.0).is_ok());
        assert!(matches!(
            validate_aperture(0.0),
            Err(CameraExifError::InvalidAperture { value: 0.0 })
        ));
        assert!(matches!(
            validate_aperture(-2.0),
            Err(CameraExifError::InvalidAperture { value: -2.0 })
        ));
    }

    /// T-006-b: stamp ISO, FocalLength, and FNumber on a fixture JPEG.
    #[test]
    fn t_006_b_stamps_iso_focal_aperture_on_jpeg() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("frame.jpg");
        write_fixture_jpeg(&path);

        stamp_ingest_metadata(&path, "Canon AE-1", Some(400), Some(50.0), Some(2.4))
            .expect("stamp ingest metadata");

        let meta = Metadata::new_from_path(&path).expect("read stamped jpeg");
        assert_eq!(first_make(&meta).as_deref(), Some("Canon"));
        assert_eq!(first_model(&meta).as_deref(), Some("AE-1"));
        assert_eq!(first_iso(&meta), Some(400));
        assert_eq!(first_iso_speed(&meta), Some(400));
        let focal = first_focal_length(&meta).expect("focal length tag");
        assert!((focal - 50.0).abs() < 0.01, "focal length {focal}");
        let f_number = first_f_number(&meta).expect("f-number tag");
        assert!((f_number - 2.4).abs() < 0.01, "f-number {f_number}");
    }

    /// Bare / post-rotate JPEGs have no APP1 — must still stamp (job_id=4 failure mode).
    #[test]
    fn stamps_jpeg_with_no_existing_exif_after_rotate_rewrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("imm000_00.jpg");
        write_fixture_jpeg(&path);

        assert!(
            Metadata::new_from_path(&path).is_err(),
            "fixture should have no EXIF before stamp"
        );

        stamp_ingest_metadata(&path, "Canon AE-1", Some(400), None, None)
            .expect("stamp onto bare jpeg");

        crate::image_rotate::rotate_cw(&path).expect("rotate rewrite strips EXIF");
        assert!(
            Metadata::new_from_path(&path).is_err(),
            "rotate rewrite should leave no EXIF"
        );

        stamp_ingest_metadata(&path, "Leica M6", Some(200), Some(50.0), Some(2.4))
            .expect("stamp after rotate rewrite");

        let meta = Metadata::new_from_path(&path).expect("read re-stamped jpeg");
        assert_eq!(first_make(&meta).as_deref(), Some("Leica"));
        assert_eq!(first_model(&meta).as_deref(), Some("M6"));
        assert_eq!(first_iso(&meta), Some(200));
    }
}
