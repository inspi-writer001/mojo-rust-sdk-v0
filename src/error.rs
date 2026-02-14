use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorldError {
    #[error("RPC Error: {0}")]
    RpcError(String),
    #[error("Serialization Error")]
    SerializationError,
    #[cfg(feature = "arweave")]
    #[error("Image upload failed: {0}")]
    ImageUploadError(String),
    #[cfg(feature = "image-upload")]
    #[error("Invalid image format: {0}. Supported formats: PNG, JPG, GIF, WebP")]
    InvalidImageFormat(String),
    #[cfg(feature = "image-upload")]
    #[error("Image too large: {0} bytes (max: {1} bytes)")]
    ImageTooLarge(usize, usize),
    #[error("Failed to download image from URL: {0}")]
    ImageDownloadError(String),
    #[cfg(feature = "native")]
    #[error("Failed to read image file: {0}")]
    ImageReadError(String),
    #[error("NFT creation failed: {0}")]
    NftCreationError(String),
    #[cfg(feature = "arweave")]
    #[error("Metadata upload failed: {0}")]
    MetadataUploadError(String),
    #[cfg(feature = "native")]
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    #[cfg(feature = "native")]
    #[error("Invalid asset data: {0}")]
    InvalidAssetData(String),
    #[error("Failed to fetch metadata: {0}")]
    MetadataFetchError(String),
    #[cfg(feature = "native")]
    #[error("Failed to deserialize asset: {0}")]
    AssetDeserializationError(String),
    #[cfg(feature = "native")]
    #[error("Collection error: {0}")]
    CollectionError(String),
    #[cfg(feature = "native")]
    #[error("Character not found: {0}")]
    CharacterNotFound(String),
}
