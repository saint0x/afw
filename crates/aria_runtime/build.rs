fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../quilt/proto/quilt.proto");

    tonic_build::configure()
        .type_attribute("quilt.ContainerInfo", "#[derive(serde::Serialize)]")
        .type_attribute("quilt.GetSystemMetricsResponse", "#[derive(serde::Serialize)]")
        .type_attribute("quilt.NetworkNode", "#[derive(serde::Serialize)]")
        .type_attribute("quilt.GetContainerNetworkInfoResponse", "#[derive(serde::Serialize)]")
        .compile_protos(&["../../crates/quilt/proto/quilt.proto"], &["../../crates/quilt/proto/"])?;

    Ok(())
} 