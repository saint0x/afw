fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../quilt/proto/quilt.proto");
    println!("cargo:rerun-if-changed=proto/aria.proto");

    // Compile the Quilt daemon proto
    tonic_build::configure()
        .type_attribute("quilt.ContainerInfo", "#[derive(serde::Serialize)]")
        .type_attribute("quilt.GetSystemMetricsResponse", "#[derive(serde::Serialize)]")
        .type_attribute("quilt.NetworkNode", "#[derive(serde::Serialize)]")
        .type_attribute("quilt.GetContainerNetworkInfoResponse", "#[derive(serde::Serialize)]")
        .compile_protos(&["../../crates/quilt/proto/quilt.proto"], &["../../crates/quilt/proto/"])?;

    // Compile the Aria Runtime API proto
    tonic_build::configure()
        .type_attribute("aria.Task", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("aria.Session", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("aria.Container", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("aria.Notification", "#[derive(serde::Serialize, serde::Deserialize)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/aria.proto"], &["proto/"])?;

    Ok(())
} 