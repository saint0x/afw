fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../quilt/proto/quilt.proto");

    tonic_build::configure()
        .build_server(false) // We only need the client
        .compile(
            &["../quilt/proto/quilt.proto"],
            &["../quilt/proto"],
        )?;

    Ok(())
} 