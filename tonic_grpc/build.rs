use std::io::Result;

fn main() -> Result<()> {
    // tonic_build::compile_protos("protos/voting.proto")?;
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("protos")
        .compile(&["protos/voting.proto"], &["protos"])?;
    Ok(())
}
