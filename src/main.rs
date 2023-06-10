//! This is the `viguno` app.
#![deny(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![warn(missing_docs)]

pub mod algos;
pub mod common;
pub mod convert;
pub mod pbs;
pub mod query;
pub mod server;
pub mod simulate;

use clap::{Parser, Subcommand};

/// CLI parser based on clap.
#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "VarFish component for phenotypes/diseases",
    long_about = "Viguno (Versatile Interface for Genetics Utilization of Nice Ontologies) \
    provides the REST API for phenotype/disease information and association to \
    diseases"
)]
struct Cli {
    /// Commonly used arguments
    #[command(flatten)]
    common: common::Args,

    /// The sub command to run
    #[command(subcommand)]
    command: Commands,
}

/// Enum supporting the parsing of sub commands.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Commands {
    Convert(crate::convert::Args),
    Query(crate::query::Args),
    RunServer(crate::server::Args),
    Simulate(crate::simulate::Args),
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // Build a tracing subscriber according to the configuration in `cli.common`.
    let collector = tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(match cli.common.verbose.log_level() {
            Some(level) => match level {
                log::Level::Error => tracing::Level::ERROR,
                log::Level::Warn => tracing::Level::WARN,
                log::Level::Info => tracing::Level::INFO,
                log::Level::Debug => tracing::Level::DEBUG,
                log::Level::Trace => tracing::Level::TRACE,
            },
            None => tracing::Level::INFO,
        })
        .compact()
        .finish();

    // Install collector and go into sub commands.
    tracing::subscriber::with_default(collector, || {
        match &cli.command {
            Commands::Convert(args) => {
                convert::run(&cli.common, args)?;
            }
            Commands::Query(args) => {
                query::run(&cli.common, args)?;
            }
            Commands::RunServer(args) => {
                server::run(&cli.common, args)?;
            }
            Commands::Simulate(args) => {
                simulate::run(&cli.common, args)?;
            }
        }

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
