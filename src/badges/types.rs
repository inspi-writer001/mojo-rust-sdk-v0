use solana_pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct BadgeCollection {
    pub collection: Pubkey,
    pub update_authority: Pubkey,
}

#[derive(Debug, Clone)]
pub struct Badge {
    pub asset: Pubkey,
    pub collection: Pubkey,
    pub owner: Pubkey,
    pub qualifying_action: QualifyingAction,
}

#[derive(Debug, Clone)]
pub struct BadgeTemplate {
    pub asset: Pubkey,
    pub collection: Pubkey,
    pub name: String,
    pub uri: String,
    pub qualifying_action: QualifyingAction,
}

#[derive(Debug, Clone)]
pub struct BadgeMint {
    pub signature: solana_sdk::signature::Signature,
    pub badge: Pubkey,
    pub owner: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum QualifyingAction {
    Named(String),
    Id(u32),
}

impl QualifyingAction {
    pub fn named(value: impl Into<String>) -> Self {
        Self::Named(value.into())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BadgeCollectionMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub properties: BadgeMetadataProperties,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BadgeMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub properties: BadgeMetadataProperties,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BadgeMetadataProperties {
    #[serde(rename = "type")]
    pub asset_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifying_action: Option<QualifyingAction>,
}

impl BadgeCollectionMetadata {
    pub fn new(name: &str, description: Option<&str>, image_uri: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.unwrap_or("Badge collection").to_string(),
            image: image_uri.to_string(),
            properties: BadgeMetadataProperties {
                asset_type: "badge_collection".to_string(),
                qualifying_action: None,
            },
        }
    }
}

impl BadgeMetadata {
    pub fn new(
        name: &str,
        description: Option<&str>,
        image_uri: &str,
        qualifying_action: QualifyingAction,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.unwrap_or("Badge").to_string(),
            image: image_uri.to_string(),
            properties: BadgeMetadataProperties {
                asset_type: "badge".to_string(),
                qualifying_action: Some(qualifying_action),
            },
        }
    }
}
