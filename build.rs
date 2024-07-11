use std::{env, path::PathBuf};

fn main() -> Result<(), anyhow::Error> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("protos");
    let proto_files = vec!["viguno/v1/simulation.proto"]
        .iter()
        .map(|f| root.join(f))
        .collect::<Vec<_>>();

    // Tell cargo to recompile if any of these proto files are changed
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    let descriptor_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("proto_descriptor.bin");

    prost_build::Config::new()
        // Save descriptors to file
        .file_descriptor_set_path(&descriptor_path)
        // Add serde serialization and deserialization to the generated code.
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        // Skip serializing `None` values.
        .type_attribute(".", "#[serde_with::skip_serializing_none]")
        // Define the protobuf files to compile.
        .compile_protos(&proto_files, &[root])?;

    Ok(())
}
