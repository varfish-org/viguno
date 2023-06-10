//! Functionality shared between all commands.

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

/// Shared command line arguments.
#[derive(Parser, Debug)]
pub struct Args {
    /// Verbosity of the program
    #[clap(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}

/// Construct the `indicatif` style for progress bars.
///
/// # Panics
///
/// In the case when writing the ETA seconds could not be written to the progress bar.
pub fn indicatif_style() -> indicatif::ProgressStyle {
    let tpl = "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] \
    {human_pos}/{human_len} ({per_sec})";
    indicatif::ProgressStyle::with_template(tpl)
        .unwrap()
        .with_key(
            "eta",
            |state: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64())
                    .expect("could not write the ETA as seconds to progress bar");
            },
        )
        .progress_chars("#>-")
}

/// Construct an `indicatif` progress bar with the common style.
///
/// Also, we will enable a steady tick every 0.1s and hide in tests.
pub fn progress_bar(#[allow(unused_variables)] len: usize) -> indicatif::ProgressBar {
    #[cfg(test)]
    let pb = indicatif::ProgressBar::hidden();
    #[cfg(not(test))]
    let pb = indicatif::ProgressBar::new(len as u64).with_style(indicatif_style());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Load HPO either from binary `$path_hpo/hpo.bin` if it exist, otherwise load as
/// standard directory from `$path_hpo`.
///
/// # Errors
///
/// In the case of loading failure.
pub fn load_hpo<P: AsRef<std::path::Path>>(path: P) -> Result<hpo::Ontology, anyhow::Error> {
    if path.as_ref().join("hpo.bin").exists() {
        tracing::info!(
            "  attempting to load binary HPO file from {}",
            path.as_ref().display()
        );
        Ok(hpo::Ontology::from_binary(path.as_ref().join("hpo.bin"))?)
    } else {
        tracing::info!(
            "  attempting to load HPO from standard file {}",
            path.as_ref().display()
        );
        Ok(hpo::Ontology::from_standard(&format!(
            "{}",
            path.as_ref().display()
        ))?)
    }
}
