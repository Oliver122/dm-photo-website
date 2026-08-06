//! JPEG rotation for REQ-007 preview. Full tests live on `feat/dm-analog-preview-slice-rotate`.
//! TODO(REQ-007): replace with rotate-slice implementation when merged.

use std::path::Path;

use anyhow::Context;

/// Rotate image 90° clockwise and rewrite file on disk.
pub fn rotate_cw(path: &Path) -> anyhow::Result<()> {
    let img = image::open(path).with_context(|| format!("open image {}", path.display()))?;
    img.rotate90()
        .save(path)
        .with_context(|| format!("save rotated image {}", path.display()))?;
    Ok(())
}

/// Rotate image 90° counter-clockwise and rewrite file on disk.
pub fn rotate_ccw(path: &Path) -> anyhow::Result<()> {
    let img = image::open(path).with_context(|| format!("open image {}", path.display()))?;
    img.rotate270()
        .save(path)
        .with_context(|| format!("save rotated image {}", path.display()))?;
    Ok(())
}
