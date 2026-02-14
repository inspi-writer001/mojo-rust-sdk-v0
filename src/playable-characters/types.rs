use solana_pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct CharacterCollection {
    pub collection: Pubkey,
    pub owner: Pubkey,
}

#[derive(Debug, Clone)]
pub struct Character {
    pub asset: Pubkey,
    pub collection: Pubkey,
    pub owner: Pubkey,
}

#[derive(Debug, Clone)]
pub struct CharacterData {
    pub asset: Pubkey,
    pub collection: Option<Pubkey>,
    pub owner: Pubkey,
    pub name: String,
    pub description: String,
    pub image_uri: String,
    pub metadata_uri: String,
}

#[derive(Debug, Clone)]
pub struct CollectionData {
    pub collection: Pubkey,
    pub update_authority: Pubkey,
    pub name: String,
    pub uri: String,
    pub num_minted: u32,
    pub current_size: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CharacterMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub properties: CharacterMetadataProperties,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CharacterMetadataProperties {
    #[serde(rename = "type")]
    pub asset_type: String,
}

impl CharacterMetadata {
    pub fn new(name: &str, description: Option<&str>, image_uri: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.unwrap_or("Playable character NFT").to_string(),
            image: image_uri.to_string(),
            properties: CharacterMetadataProperties {
                asset_type: "character".to_string(),
            },
        }
    }
}
