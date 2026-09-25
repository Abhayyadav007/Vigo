//! Image storage for catalog uploads.

use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
/// Public URL prefix for locally stored media (served by `ServeDir`).
pub const LOCAL_URL_PREFIX: &str = "/media";

#[derive(Debug, Clone)]
pub enum MediaStore {
    /// Files on local disk, served at `/media/...`. Fine for dev and a single
    /// instance; not shared across instances.
    Local { dir: PathBuf },
    // TODO(phase-8): S3-compatible object storage (Cloudflare R2) for production.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Jpeg,
    Png,
    Webp,
}

impl ImageKind {
    /// Detects the format from magic bytes; the client's filename and
    /// Content-Type are not trusted.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0xFF, 0xD8, 0xFF, ..] => Some(Self::Jpeg),
            [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some(Self::Png),
            [
                b'R',
                b'I',
                b'F',
                b'F',
                _,
                _,
                _,
                _,
                b'W',
                b'E',
                b'B',
                b'P',
                ..,
            ] => Some(Self::Webp),
            _ => None,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

/// Stores an uploaded image and returns its public URL.
pub async fn save_image(state: &AppState, bytes: &[u8]) -> AppResult<String> {
    if bytes.is_empty() {
        return Err(AppError::Validation("file is empty".into()));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Validation("image must be 5 MB or smaller".into()));
    }
    let kind = ImageKind::sniff(bytes)
        .ok_or_else(|| AppError::Validation("only JPEG, PNG or WebP images are allowed".into()))?;
    let key = format!(
        "{}/{}.{}",
        Utc::now().format("%Y/%m"),
        Uuid::new_v4().simple(),
        kind.extension()
    );

    // TODO(phase-8): garbage-collect uploads that no product/category references.
    match state.media.as_ref() {
        MediaStore::Local { dir } => {
            let path = dir.join(&key);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(anyhow::Error::from)?;
            }
            // Write then rename so a half-written file is never served.
            let tmp = path.with_extension("part");
            tokio::fs::write(&tmp, bytes)
                .await
                .map_err(anyhow::Error::from)?;
            tokio::fs::rename(&tmp, &path)
                .await
                .map_err(anyhow::Error::from)?;
            Ok(format!("{LOCAL_URL_PREFIX}/{key}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImageKind;

    #[test]
    fn sniffs_by_content() {
        assert_eq!(
            ImageKind::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(
            ImageKind::sniff(b"\x89PNG\r\n\x1a\n...."),
            Some(ImageKind::Png)
        );
        assert_eq!(
            ImageKind::sniff(b"RIFF\0\0\0\0WEBPVP8 "),
            Some(ImageKind::Webp)
        );
        assert_eq!(ImageKind::sniff(b"<svg xmlns="), None);
        assert_eq!(ImageKind::sniff(b"GIF89a"), None);
    }
}
