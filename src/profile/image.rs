use crate::error::WorldError;
use anyhow::{ensure, Result};

pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

#[cfg(feature = "native")]
pub async fn load_image_data(source: &crate::profile::types::ImageSource) -> Result<Vec<u8>> {
    match source {
        crate::profile::types::ImageSource::LocalFile(path) => tokio::fs::read(path)
            .await
            .map_err(|e| WorldError::ImageReadError(format!("Failed to read file: {}", e)).into()),
        crate::profile::types::ImageSource::Url(url) => {
            let response = reqwest::get(url).await.map_err(|e| {
                WorldError::ImageDownloadError(format!("Failed to download: {}", e))
            })?;

            ensure!(
                response.status().is_success(),
                WorldError::ImageDownloadError(format!("HTTP error: {}", response.status()))
            );

            response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
                WorldError::ImageDownloadError(format!("Failed to read response: {}", e)).into()
            })
        }
    }
}

pub fn validate_image(image_data: &[u8]) -> Result<()> {
    ensure!(
        image_data.len() <= MAX_IMAGE_SIZE,
        WorldError::ImageTooLarge(image_data.len(), MAX_IMAGE_SIZE)
    );

    let _img = image::load_from_memory(image_data).map_err(|e| {
        WorldError::InvalidImageFormat(format!(
            "Invalid image format: {}. Supported: PNG, JPG, GIF, WebP",
            e
        ))
    })?;

    Ok(())
}
