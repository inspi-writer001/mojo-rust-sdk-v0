use anyhow::{Context, Result};
use arweave_rs::Arweave;
use std::path::PathBuf;
use url::Url;

pub struct ArweaveUploader {
    wallet_path: Option<String>,
    gateway_url: String,
}

impl ArweaveUploader {
    pub fn new(wallet_path: Option<String>, gateway_url: Option<String>) -> Self {
        Self {
            wallet_path,
            gateway_url: gateway_url.unwrap_or_else(|| "https://arweave.net".to_string()),
        }
    }

    pub async fn upload(&self, data: &[u8], content_type: Option<&str>) -> Result<String> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Get wallet path
        let wallet_path = self.get_wallet_path()?;
        let wallet_path_buf = PathBuf::from(&wallet_path);

        // Parse gateway URL
        let base_url = Url::parse(&self.gateway_url).context("Invalid Arweave gateway URL")?;

        // Initialize Arweave client with wallet
        let arweave = Arweave::from_keypair_path(wallet_path_buf, base_url)
            .context("Failed to initialize Arweave client. Ensure wallet file is valid")?;

        // Create a temporary file with the data
        let mut temp_file = NamedTempFile::new().context("Failed to create temporary file")?;
        temp_file
            .write_all(data)
            .context("Failed to write data to temporary file")?;
        let temp_path = temp_file.path().to_path_buf();
        temp_file.flush()?;

        use arweave_rs::crypto::base64::Base64;
        use arweave_rs::transaction::tags::Tag;

        let mut tags = Vec::new();
        if let Some(ct) = content_type {
            let name = Base64::from("Content-Type".as_bytes());
            let value = Base64::from(ct.as_bytes());
            tags.push(Tag { name, value });
        }

        // Get the fee estimate
        let target = Base64::from("".as_bytes());
        let fee = arweave
            .get_fee(target, data.to_vec())
            .await
            .context("Failed to get Arweave fee estimate")?;

        // Upload file
        let (tx_id, _fee_paid) = arweave
            .upload_file_from_path(temp_path, tags, fee)
            .await
            .context("Failed to upload file to Arweave")?;

        Ok(tx_id)
    }

    fn get_wallet_path(&self) -> Result<String> {
        if let Some(path) = &self.wallet_path {
            if std::path::Path::new(path).exists() {
                return Ok(path.clone());
            }
        }

        if let Ok(path) = std::env::var("ARWEAVE_WALLET") {
            if std::path::Path::new(&path).exists() {
                return Ok(path);
            }
        }

        let default_path = dirs::home_dir()
            .map(|h| h.join(".arweave").join("wallet.json"))
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        if default_path.exists() {
            return Ok(default_path.to_string_lossy().to_string());
        }

        Err(anyhow::anyhow!(
            "Arweave wallet not found. Please provide wallet path or set ARWEAVE_WALLET environment variable"
        ))
    }

    pub fn uri_from_tx_id(&self, tx_id: &str) -> String {
        format!("{}/{}", self.gateway_url, tx_id)
    }
}

impl Default for ArweaveUploader {
    fn default() -> Self {
        Self::new(None, None)
    }
}
