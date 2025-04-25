fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/metadata.proto")?;
    // println!("cargo:rerun-if-changed=src/proto/metadata.proto");
    // println!("cargo:include={}", "src/proto"); // ensures proper include paths
    Ok(())
}