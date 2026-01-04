use solana_pubkey::Pubkey;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ImageSource {
    LocalFile(PathBuf),
    Url(String),
}

impl ImageSource {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self::LocalFile(path.into())
    }

    pub fn from_url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }
}

#[derive(Debug, Clone)]
pub struct ProfilePicture {
    pub asset: Pubkey,
    pub collection: Option<Pubkey>,
    pub owner: Pubkey,
}

#[derive(serde::Serialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub properties: MetadataProperties,
}

#[derive(serde::Serialize)]
pub struct MetadataProperties {
    #[serde(rename = "type")]
    pub asset_type: String,
}

impl Metadata {
    pub fn new(name: &str, description: Option<&str>, image_uri: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.unwrap_or("Profile picture NFT").to_string(),
            image: image_uri.to_string(),
            properties: MetadataProperties {
                asset_type: "profile_picture".to_string(),
            },
        }
    }
}
