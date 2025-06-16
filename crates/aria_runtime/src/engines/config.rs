use serde::Deserialize;

/// Configuration for the Quilt container service client.
#[derive(Debug, Clone, Deserialize)]
pub struct QuiltConfig {
    /// The gRPC endpoint of the `quiltd` daemon.
    /// Example: "http://127.0.0.1:50051"
    pub endpoint: String,
}
