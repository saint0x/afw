// Quilt Library Interface
// Exposes proto module for external consumption

pub mod proto {
    tonic::include_proto!("quilt");
}

pub mod bundle_runtime;
pub mod bun_controller;

pub use bundle_runtime::{
    BundleWorkspaceManager, BundleWorkspaceConfig, ResourceLimits, 
    NetworkConfig, IsolationLevel, BundleRuntimeError
};
pub use bun_controller::{
    BunController, BunRuntimeConfig, BunExecutionResult, 
    BunHealthStatus, BunControllerError
}; 