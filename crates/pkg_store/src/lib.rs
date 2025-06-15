/*!
# Package Store

Content-addressed storage and verification for .aria bundles.
Handles signature verification and bundle management with custom storage.
*/


use std::collections::HashMap;

/// Result type for package store operations
pub type PkgResult<T> = Result<T, PkgError>;

/// Package store specific errors
#[derive(Debug, thiserror::Error)]
pub enum PkgError {
    #[error("Bundle not found: {0}")]
    BundleNotFound(String),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(String),
}

pub struct PackageStore {
    // In-memory storage for now - we'll add custom disk persistence later
    bundles: HashMap<String, Vec<u8>>,
    signatures: HashMap<String, Vec<u8>>,
}

impl PackageStore {
    pub async fn new() -> PkgResult<Self> {
        Ok(Self {
            bundles: HashMap::new(),
            signatures: HashMap::new(),
        })
    }

    pub async fn store_bundle(&mut self, bundle_data: Vec<u8>) -> PkgResult<String> {
        // TODO: Verify signature, compute hash, store bundle
        let bundle_hash = blake3::hash(&bundle_data).to_hex().to_string();
        self.bundles.insert(bundle_hash.clone(), bundle_data);
        Ok(bundle_hash)
    }

    pub async fn get_bundle(&self, bundle_hash: &str) -> PkgResult<Option<Vec<u8>>> {
        Ok(self.bundles.get(bundle_hash).cloned())
    }

    pub async fn verify_bundle(&self, _bundle_data: &[u8]) -> PkgResult<bool> {
        // TODO: Verify bundle signature using ed25519-dalek
        Ok(true)
    }

    pub async fn list_bundles(&self) -> PkgResult<Vec<String>> {
        Ok(self.bundles.keys().cloned().collect())
    }

    pub async fn delete_bundle(&mut self, bundle_hash: &str) -> PkgResult<()> {
        self.bundles.remove(bundle_hash);
        self.signatures.remove(bundle_hash);
        Ok(())
    }
} 