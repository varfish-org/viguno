//! Code for running the precomputation.

use indicatif::ParallelProgressIterator;
use prost::Message;
use rayon::prelude::*;
use rocksdb::{DBWithThreadMode, MultiThreaded};
use std::io::Write;
use std::time::Instant;

use clap::Parser;
use hpo::{
    annotations::AnnotationId,
    similarity::{Builtins, GroupSimilarity, StandardCombiner},
    term::{HpoGroup, InformationContentKind},
    HpoSet, HpoTermId, Ontology,
};

use crate::{
    common::{IcBasedOn, ScoreCombiner, SimilarityMethod},
    pbs::simulation::SimulationResults,
};

/// Command line arguments for Viguno.
#[derive(Parser, Debug)]
#[command(author, version, about = "Prepare values for Viguno", long_about = None)]
pub struct Args {
    /// Path to the directory with the HPO files.
    #[arg(long, required = true)]
    pub path_hpo_dir: String,
    /// Path to output RocksDB.
    #[arg(long, required = true)]
    pub path_out_rocksdb: String,

    /// Number of simulations to perform for each gene and term set size.
    #[arg(long, default_value_t = 100_000, value_parser = clap::value_parser!(u64).range(2..))]
    pub num_simulations: u64,
    /// Run simulations for `min_terms..=max_terms` terms.
    #[arg(long, default_value_t = 1)]
    pub min_terms: usize,
    /// Run simulations for `min_terms..=max_terms` terms.
    #[arg(long, default_value_t = 10)]
    pub max_terms: usize,

    /// What should information content be based on.
    #[arg(long)]
    pub ic_base: IcBasedOn,
    /// The similarity method to use.
    #[arg(long)]
    pub similarity: SimilarityMethod,
    /// The score combiner.
    #[arg(long)]
    pub combiner: ScoreCombiner,

    /// Optional gene ID or symbol to limit to.
    #[arg(long)]
    pub only_gene: Option<String>,
    /// Optional path to folder with per-gene logs.
    #[arg(long)]
    pub path_gene_logs: Option<String>,

    /// Number of threads to use for simulation (default is 1 thread per core).
    #[arg(long)]
    pub num_threads: Option<usize>,
    /// Seed for the random number generator.
    #[arg(long)]
    pub seed: Option<u64>,
}

/// Run simulation using ontology and number of terms.
fn run_simulation(
    db: &DBWithThreadMode<MultiThreaded>,
    ontology: &Ontology,
    args: &Args,
    num_terms: usize,
) -> Result<(), anyhow::Error> {
    tracing::info!("  running simulation for {} terms ...", num_terms);
    let before = Instant::now();

    // We want at least two simulations.
    let num_simulations = std::cmp::max(args.num_simulations, 2);

    // Get all HPO terms for phenotypic abnormalities.
    let hpo_abnormality = ontology
        .hpo(HpoTermId::from(String::from("HP:0000118")))
        .ok_or(anyhow::anyhow!(
            "could not find HP:0000118 (phenotypic abnormality)"
        ))?;
    let term_ids = ontology
        .hpos()
        .filter(|t| t.child_of(&hpo_abnormality))
        .map(|t| t.id())
        .collect::<Vec<_>>();

    // Get all genes into a vector so we can use parallel iteration.
    let genes = {
        let mut genes = ontology.genes().collect::<Vec<_>>();
        if let Some(only_gene) = args.only_gene.as_ref() {
            genes.retain(|g| {
                g.id().to_string().as_str().eq(only_gene.as_str())
                    || g.symbol().eq(only_gene.as_str())
            });
        }
        genes
    };

    // The pairwise term simliarity score to use.
    let pairwise_sim = Builtins::Resnik(InformationContentKind::Gene);
    // The combiner for multiple pairwise scores.
    let combiner: StandardCombiner = args.combiner.into();
    // The groupwise similarity to use.
    let group_sim = GroupSimilarity::new(combiner, pairwise_sim);

    // Run simulations for each gene in parallel.
    genes
        .par_iter()
        .progress_with(crate::common::progress_bar(genes.len()))
        .for_each(|gene| {
            let mut log_file = if let Some(path_gene_logs) = args.path_gene_logs.as_ref() {
                let path = std::path::Path::new(path_gene_logs).join(num_terms.to_string());
                std::fs::create_dir_all(&path).expect("cannot create logs directory");
                Some(
                    std::fs::File::create(format!("{}/{}.txt", path.display(), gene.symbol()))
                        .expect("could not open file"),
                )
            } else {
                None
            };

            // Obtain `HpoSet` from gene.
            let gene_terms = HpoSet::new(
                ontology,
                gene.to_hpo_set(ontology)
                    .child_nodes()
                    .without_modifier()
                    .into_iter()
                    .collect::<HpoGroup>(),
            );

            // Obtain sorted list of similarity scores from simulations.
            let mut scores = (0..num_simulations)
                .map(|_| {
                    // Pick `num_terms` random terms with circuit breakers on number of tries.
                    let max_tries = 1000;
                    let sampled_terms = {
                        let mut tries = 0;
                        let mut hpo_group = HpoGroup::new();
                        while hpo_group.len() < num_terms {
                            tries += 1;
                            assert!(tries <= max_tries, "tried too often to pick random terms");
                            let term_id = term_ids[fastrand::usize(0..term_ids.len())];
                            if !hpo_group.contains(&term_id) {
                                hpo_group.insert(term_id);
                            }
                        }
                        HpoSet::new(ontology, hpo_group)
                    };

                    // Compute the similarity from the sampled terms to the terms from the gene.
                    let res_score = group_sim.calculate(&sampled_terms, &gene_terms);

                    if let Some(log_file) = log_file.as_mut() {
                        writeln!(
                            log_file,
                            "{}\t{}\t{}",
                            res_score,
                            gene.symbol(),
                            sampled_terms
                                .iter()
                                .map(|t| format!("{} ({})", t.id(), t.name()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                        .expect("could not write");
                    }

                    res_score
                })
                .collect::<prost::alloc::vec::Vec<_>>();

            // Sort the scores ascendingly.
            scores.sort_by(|a, b| a.partial_cmp(b).expect("NaN value"));

            // Copy the scores into the score distribution.
            let ncbi_gene_id = gene.id().as_u32();
            let sim_res = SimulationResults {
                ncbi_gene_id,
                gene_symbol: gene.name().to_string(),
                term_count: num_terms.try_into().expect("too many terms"),
                scores,
            };

            // Encode as byte array.
            let sim_res = sim_res.encode_to_vec();

            // Write to RocksDB.
            let cf_resnik = db.cf_handle("scores").unwrap();
            let key = format!("{ncbi_gene_id}:{num_terms}");
            db.put_cf(&cf_resnik, key.as_bytes(), sim_res)
                .expect("writing to RocksDB failed");
        });
    tracing::info!("  ... done in {:?}", before.elapsed());

    Ok(())
}

/// Main entry point for `prepare` command.
///
/// # Errors
///
/// In the case that there is an error in running the preparation command.
pub fn run(args_common: &crate::common::Args, args: &Args) -> Result<(), anyhow::Error> {
    tracing::info!("args_common = {:?}", &args_common);
    tracing::info!("args = {:?}", &args);

    if let Some(level) = args_common.verbose.log_level() {
        match level {
            log::Level::Trace | log::Level::Debug => {
                std::env::set_var("RUST_LOG", "debug");
                env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
            }
            _ => (),
        }
    }

    tracing::info!("Loading HPO...");
    let before_loading = Instant::now();
    let ontology = crate::common::load_hpo(&args.path_hpo_dir)?;
    tracing::info!("...done loading HPO in {:?}", before_loading.elapsed());

    tracing::info!("Opening RocksDB for writing...");
    let before_rocksdb = Instant::now();
    let options = rocksdb_utils_lookup::tune_options(rocksdb::Options::default(), None);
    let cf_names = &["meta", "scores"];
    let db = rocksdb::DB::open_cf_with_opts(
        &options,
        &args.path_out_rocksdb,
        cf_names
            .iter()
            .map(|name| ((*name).to_string(), options.clone()))
            .collect::<Vec<_>>(),
    )?;
    // write out metadata
    let cf_meta = db
        .cf_handle("meta")
        .ok_or(anyhow::anyhow!("column family meta not found"))?;
    db.put_cf(&cf_meta, "hpo-version", ontology.hpo_version())?;
    db.put_cf(&cf_meta, "app-version", crate::common::version())?;
    tracing::info!("...done opening RocksDB in {:?}", before_rocksdb.elapsed());

    tracing::info!("Running simulations...");
    let before_simulations = Instant::now();
    if let Some(seed) = args.seed {
        fastrand::seed(seed);
    }
    if let Some(num_threds) = args.num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threds)
            .build_global()?;
    }
    for num_terms in args.min_terms..=args.max_terms {
        run_simulation(&db, &ontology, args, num_terms)?;
    }
    tracing::info!(
        "... done with simulations in {:?}",
        before_simulations.elapsed()
    );

    tracing::info!("Enforcing manual compaction");
    rocksdb_utils_lookup::force_compaction_cf(&db, cf_names, Some("  "), true)?;
    tracing::info!("All done. Have a nice day!");
    Ok(())
}

#[cfg(test)]
mod test {
    use clap_verbosity_flag::Verbosity;
    use temp_testdir::TempDir;

    use crate::common::{IcBasedOn, ScoreCombiner, SimilarityMethod};

    #[test]
    fn smoke_test_run() -> Result<(), anyhow::Error> {
        let tmp_dir = TempDir::default();

        let args_common = crate::common::Args {
            verbose: Verbosity::new(0, 0),
        };
        let args = super::Args {
            path_hpo_dir: String::from("tests/data/hpo"),
            path_out_rocksdb: format!("{}", tmp_dir.display()),
            num_simulations: 2,
            min_terms: 1,
            max_terms: 10,
            only_gene: Some(String::from("TGDS")),
            path_gene_logs: None,
            num_threads: None,
            seed: Some(42),
            ic_base: IcBasedOn::default(),
            similarity: SimilarityMethod::default(),
            combiner: ScoreCombiner::default(),
        };

        super::run(&args_common, &args)
    }
}
