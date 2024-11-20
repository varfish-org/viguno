//! Conversion of HPO text files to `hpo` binary format.

use std::io::Write;

use clap::Parser;

/// Command line arguments for `convert` sub command.
#[derive(Parser, Debug)]
#[command(author, version, about = "Convert HPO text files to binary format", long_about = None)]
pub struct Args {
    /// Path to the directory with the HPO files.
    #[arg(long, required = true)]
    pub path_hpo_dir: String,
    /// Path to the output binary file.
    #[arg(long, required = true)]
    pub path_out_bin: String,
}

/// Main entry point for `convert` command.
///
/// # Errors
///
/// In the case of query execution failure.
///
/// # Panics
///
/// In the case of term lookup failure.
pub fn run(args_common: &crate::common::Args, args: &Args) -> Result<(), anyhow::Error> {
    tracing::info!("args_common = {:?}", &args_common);
    tracing::info!("args = {:?}", &args);

    if let Some(log::Level::Trace | log::Level::Debug) = args_common.verbose.log_level() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    }

    tracing::info!("Loading HPO...");
    let before_loading = std::time::Instant::now();
    let hpo = crate::common::load_hpo(&args.path_hpo_dir)?;
    tracing::info!("...done loading HPO in {:?}", before_loading.elapsed());
    tracing::info!("Ontology [{}] with {} terms", hpo.hpo_version(), hpo.len());

    tracing::info!("Writing binary file...");
    let before_writing = std::time::Instant::now();
    let filename = &args.path_out_bin;
    let mut fh = std::fs::File::create(filename).expect("Cannot create file");
    fh.write_all(&hpo.as_bytes())?;
    tracing::info!("...done writing binary in {:?}", before_writing.elapsed());

    tracing::info!("All done. Have a nice day!");

    Ok(())
}
