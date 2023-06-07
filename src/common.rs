//! Functionality shared between all commands.

use clap_verbosity_flag::{InfoLevel, Verbosity};
use clap::Parser;

/// Shared command line arguments.
#[derive(Parser, Debug)]
pub struct Args {
    /// Verbosity of the program
    #[clap(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}
