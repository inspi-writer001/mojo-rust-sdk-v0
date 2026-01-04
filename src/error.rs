use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorldError {
    #[error("RPC Error: {0}")]
    RpcError(String),
    #[error("Serialization Error")]
    SerializationError,
    #[error("Image upload failed: {0}")]
    ImageUploadError(String),
    #[error("Invalid image format: {0}. Supported formats: PNG, JPG, GIF, WebP")]
    InvalidImageFormat(String),
    #[error("Image too large: {0} bytes (max: {1} bytes)")]
    ImageTooLarge(usize, usize),
    #[error("Failed to download image from URL: {0}")]
    ImageDownloadError(String),
    #[error("Failed to read image file: {0}")]
    ImageReadError(String),
    #[error("NFT creation failed: {0}")]
    NftCreationError(String),
    #[error("Metadata upload failed: {0}")]
    MetadataUploadError(String),
}
