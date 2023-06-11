//! Functionality shared across the crate.

use std::str::FromStr;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use hpo::{
    similarity::{Builtins, StandardCombiner},
    term::InformationContentKind,
};
use strum::{EnumIter, IntoEnumIterator};

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

/// Enum for representing the information content kind.
///
/// We replicate what is in the `hpo` create so we can put them on the command line and use
/// them in HTTP queries more easily.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    EnumIter,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum IcBasedOn {
    /// Compute information content based on gene.
    #[default]
    #[display(fmt = "gene")]
    Gene,
    /// Compute information content based on OMIM disease.
    #[display(fmt = "omim")]
    Omim,
}

impl FromStr for IcBasedOn {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IcBasedOn::iter()
            .find(|m| m.to_string().as_str().eq(s))
            .ok_or(anyhow::anyhow!("unknown information content base: {}", s))
    }
}

/// Enum for representing similarity method to use.
///
/// We replicate what is in the `hpo` create so we can put them on the command line and use
/// them in HTTP queries more easily.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    EnumIter,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum SimilarityMethod {
    /// "Distance" similarity.
    #[display(fmt = "distance")]
    DistanceGene,
    /// Graph IC similarity.
    #[display(fmt = "graph-ic")]
    GraphIc,
    /// Information coefficient similarity..
    #[display(fmt = "information-coefficient")]
    InformationCoefficient,
    /// Jiang & Conrath similarity.
    #[display(fmt = "jc")]
    Jc,
    /// Lin similarity..
    #[display(fmt = "lin")]
    Lin,
    /// "Mutation" similarity.
    #[display(fmt = "mutation")]
    Mutation,
    /// "Relevance" similarity.
    #[display(fmt = "relevance")]
    Relevance,
    /// Resnik similarity..
    #[default]
    #[display(fmt = "resnik")]
    Resnik,
}

/// Convert to pairwise similarity.
pub fn to_pairwise_sim(sim: SimilarityMethod, ic_based_on: IcBasedOn) -> Builtins {
    let kind = match ic_based_on {
        IcBasedOn::Gene => InformationContentKind::Gene,
        IcBasedOn::Omim => InformationContentKind::Omim,
    };
    match sim {
        SimilarityMethod::DistanceGene => Builtins::Distance(kind),
        SimilarityMethod::GraphIc => Builtins::GraphIc(kind),
        SimilarityMethod::InformationCoefficient => Builtins::InformationCoefficient(kind),
        SimilarityMethod::Jc => Builtins::Jc(kind),
        SimilarityMethod::Lin => Builtins::Lin(kind),
        SimilarityMethod::Mutation => Builtins::Mutation(kind),
        SimilarityMethod::Relevance => Builtins::Relevance(kind),
        SimilarityMethod::Resnik => Builtins::Resnik(kind),
    }
}

impl FromStr for SimilarityMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SimilarityMethod::iter()
            .find(|m| m.to_string().as_str().eq(s))
            .ok_or(anyhow::anyhow!("unknown similarity method: {}", s))
    }
}

/// Representation of the standard combiners from HPO.
///
/// We replicate what is in the `hpo` create so we can put them on the command line and use
/// them in HTTP queries more easily.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    EnumIter,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ScoreCombiner {
    /// funSimAvg algborithm.
    #[default]
    #[display(fmt = "fun-sim-avg")]
    FunSimAvg,
    /// funSimMax algorithm.
    #[display(fmt = "fun-sim-max")]
    FunSimMax,
    /// BMA algorithm.
    #[display(fmt = "bma")]
    Bma,
}

impl From<ScoreCombiner> for StandardCombiner {
    fn from(val: ScoreCombiner) -> Self {
        match val {
            ScoreCombiner::FunSimAvg => StandardCombiner::FunSimAvg,
            ScoreCombiner::FunSimMax => StandardCombiner::FunSimMax,
            ScoreCombiner::Bma => StandardCombiner::Bwa,
        }
    }
}

impl FromStr for ScoreCombiner {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ScoreCombiner::iter()
            .find(|m| m.to_string().as_str().eq(s))
            .ok_or(anyhow::anyhow!("unknown score combiner: {}", s))
    }
}

/// The version of `viguno` package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version information that is returned by the HTTP server.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct Version {
    /// Version of the HPO.
    pub hpo: String,
    /// Version of the `viguno` package.
    pub viguno: String,
}

impl Version {
    /// Construct a new version.
    ///
    /// The viguno version is filed automatically.
    pub fn new(hpo: &str) -> Self {
        Self {
            hpo: hpo.to_string(),
            viguno: VERSION.to_string(),
        }
    }
}
