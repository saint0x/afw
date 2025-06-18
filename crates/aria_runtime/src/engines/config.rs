use serde::Deserialize;

/// Configuration for the Quilt container service client.
#[derive(Debug, Clone, Deserialize)]
pub struct QuiltConfig {
    /// The Unix socket path of the `quiltd` daemon.
    /// Example: "/run/quilt/api.sock"
    pub socket_path: String,
}
