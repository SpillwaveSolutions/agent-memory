//! Build script for memory-service.
//!
//! Compiles the protobuf definitions into Rust code using tonic-build.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: Proto compilation will be enabled in Phase 1, Plan 03
    // when the gRPC service is fully implemented.
    //
    // For now, we just ensure the proto file exists.
    println!("cargo:rerun-if-changed=../../proto/memory.proto");

    // Uncomment when ready to compile protos:
    // tonic_build::configure()
    //     .build_server(true)
    //     .build_client(true)
    //     .compile(&["../../proto/memory.proto"], &["../../proto"])?;

    Ok(())
}
