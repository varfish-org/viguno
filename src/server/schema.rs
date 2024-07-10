//! Dump schema of the REST API server.

/// Command line arguments for `server schema` sub command.
#[derive(clap::Parser, Debug)]
#[command(author, version, about = "Dump REST API schema", long_about = None)]
pub struct Args {
    /// Path to the output file.  Use stdout if missing.
    #[arg(long)]
    pub output_file: Option<String>,
}

/// Main entry point for `run-server` sub command.
///
/// # Errors
///
/// In the case that there is an error running the server.
pub fn run(args_common: &crate::common::Args, args: &Args) -> Result<(), anyhow::Error> {
    tracing::info!("args_common = {:?}", &args_common);
    tracing::info!("args = {:?}", &args);

    tracing::info!("All done. Have a nice day!");
    Ok(())
}
