use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use image::ImageReader;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageRotateError {
    #[error("invalid preview path: {detail}")]
    InvalidPath { detail: String },
    #[error("failed to read image {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: image::ImageError,
    },
    #[error("failed to write image {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: image::ImageError,
    },
}

/// Resolve `filename` under `workdir`, rejecting `..`, absolute paths, and other escapes.
pub fn resolve_job_image_path(workdir: &Path, filename: &str) -> Result<PathBuf, ImageRotateError> {
    if filename.is_empty() {
        return Err(ImageRotateError::InvalidPath {
            detail: "filename must not be empty".into(),
        });
    }

    let rel = Path::new(filename);
    if rel.is_absolute() {
        return Err(ImageRotateError::InvalidPath {
            detail: "absolute paths are not allowed".into(),
        });
    }

    for component in rel.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => {
                return Err(ImageRotateError::InvalidPath {
                    detail: "path must not contain ..".into(),
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ImageRotateError::InvalidPath {
                    detail: "path must stay under the job workdir".into(),
                });
            }
        }
    }

    Ok(workdir.join(rel))
}

/// Rotate the JPEG at `path` 90° clockwise and rewrite the file (pixel data, not EXIF-only).
pub fn rotate_jpeg_cw(path: &Path) -> Result<(), ImageRotateError> {
    rotate_jpeg(path, true)
}

/// Rotate the JPEG at `path` 90° counter-clockwise and rewrite the file (pixel data, not EXIF-only).
pub fn rotate_jpeg_ccw(path: &Path) -> Result<(), ImageRotateError> {
    rotate_jpeg(path, false)
}

/// Alias used by preview UI handlers.
pub fn rotate_cw(path: &Path) -> Result<(), ImageRotateError> {
    rotate_jpeg_cw(path)
}

/// Alias used by preview UI handlers.
pub fn rotate_ccw(path: &Path) -> Result<(), ImageRotateError> {
    rotate_jpeg_ccw(path)
}

fn rotate_jpeg(path: &Path, clockwise: bool) -> Result<(), ImageRotateError> {
    let path_str = path.display().to_string();

    let img = ImageReader::open(path)
        .map_err(|source| ImageRotateError::Read {
            path: path_str.clone(),
            source: image::ImageError::IoError(source),
        })?
        .decode()
        .map_err(|source| ImageRotateError::Read {
            path: path_str.clone(),
            source,
        })?;

    let rotated = if clockwise {
        img.rotate90()
    } else {
        img.rotate270()
    };

    let file = File::create(path).map_err(|source| ImageRotateError::Write {
        path: path_str.clone(),
        source: image::ImageError::IoError(source),
    })?;
    let mut writer = BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(&mut writer, 100);
    encoder
        .encode(
            rotated.as_bytes(),
            rotated.width(),
            rotated.height(),
            ExtendedColorType::from(rotated.color()),
        )
        .map_err(|source| ImageRotateError::Write {
            path: path_str.clone(),
            source,
        })?;
    writer
        .flush()
        .map_err(|source| ImageRotateError::Write {
            path: path_str,
            source: image::ImageError::IoError(source),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, RgbImage};

    fn write_test_jpeg(path: &Path) {
        let mut img: RgbImage = ImageBuffer::new(4, 3);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([
                (x as u8) * 40 + 10,
                (y as u8) * 50 + 20,
                ((x + y) as u8) * 30 + 5,
            ]);
        }
        let file = File::create(path).expect("create test jpeg");
        let mut writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(&mut writer, 100);
        encoder
            .encode(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8)
            .expect("encode test jpeg");
        writer.flush().expect("flush test jpeg");
    }

    /// T-007-a: four 90° CW rotations restore original pixels (in-memory math + on-disk dimensions).
    #[test]
    fn t_007_a_rotate_four_times_is_identity() {
        let mut img: RgbImage = ImageBuffer::new(4, 3);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([
                (x as u8) * 40 + 10,
                (y as u8) * 50 + 20,
                ((x + y) as u8) * 30 + 5,
            ]);
        }
        let mut rotated = img.clone();
        for _ in 0..4 {
            rotated = image::imageops::rotate90(&rotated);
        }
        assert_eq!(
            img.as_raw(),
            rotated.as_raw(),
            "four 90° CW rotations restore pixels"
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frame.jpg");
        write_test_jpeg(&path);
        let (w, h) = image::open(&path).unwrap().into_rgb8().dimensions();

        for _ in 0..4 {
            rotate_jpeg_cw(&path).expect("rotate cw on disk");
        }

        let (w2, h2) = image::open(&path).unwrap().into_rgb8().dimensions();
        assert_eq!((w, h), (w2, h2), "four on-disk CW rotations restore dimensions");
    }

    #[test]
    fn rotate_ccw_swaps_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frame.jpg");
        write_test_jpeg(&path);
        let (w, h) = image::open(&path).unwrap().into_rgb8().dimensions();

        rotate_jpeg_ccw(&path).expect("rotate ccw");

        let (w2, h2) = image::open(&path).unwrap().into_rgb8().dimensions();
        assert_eq!((h, w), (w2, h2));
    }

    /// T-007-b: path traversal and escapes are rejected.
    #[test]
    fn t_007_b_path_traversal_rejected() {
        let workdir = Path::new("/tmp/analog-ingest/42");

        for bad in [
            "../escape.jpg",
            "/etc/passwd",
            "foo/../../outside.jpg",
            "..",
            "photos/../../../etc/passwd",
        ] {
            let err = resolve_job_image_path(workdir, bad).unwrap_err();
            assert!(
                matches!(err, ImageRotateError::InvalidPath { .. }),
                "expected InvalidPath for {bad:?}, got {err:?}"
            );
        }

        let ok = resolve_job_image_path(workdir, "photos/a.jpg").unwrap();
        assert_eq!(ok, workdir.join("photos/a.jpg"));
    }
}
