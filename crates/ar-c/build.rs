fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate build metadata
    vergen::EmitBuilder::builder().all_build().all_git().emit()?;
    
    // Path to the proto file in the quilt crate
    let proto_file = "../quilt/proto/quilt.proto";
    let proto_dir = "../quilt/proto";
    
    // Generate gRPC client code (outputs to OUT_DIR by default)
    tonic_build::configure()
        .build_server(false) // We only need the client
        .build_client(true)
        .compile(&[proto_file], &[proto_dir])?;
    
    // Tell cargo to recompile if the proto file changes
    println!("cargo:rerun-if-changed={}", proto_file);
    
    Ok(())
} 