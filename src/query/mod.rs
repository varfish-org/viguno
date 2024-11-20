//! Code for ranking genes on the command line.

use hpo::similarity::Builtins;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use clap::Parser;
use hpo::{annotations::AnnotationId, term::HpoGroup, HpoTermId, Ontology};

use crate::algos::phenomizer;
use crate::query::query_result::TermDetails;

/// Command line arguments for `query` command.
#[derive(Parser, Debug)]
#[command(author, version, about = "Prepare values for `query`", long_about = None)]
pub struct Args {
    /// Path to the directory with the HPO files.
    #[arg(long, required = true)]
    pub path_hpo_dir: String,

    /// Path to JSON file with the genes to rank.
    #[arg(long)]
    pub path_genes_json: String,
    /// Path to JSON file with HPO IDs of patient.
    #[arg(long)]
    pub path_terms_json: String,
}

/// Struct for loading a gene from JSON.
#[derive(Deserialize, Debug, Clone)]
pub struct Gene {
    /// The gene symbol.
    pub gene_symbol: String,
}

/// Struct for loading an HPO term from JSON.
#[derive(Serialize, Deserialize, utoipa::ToSchema, Debug, Clone)]
pub struct HpoTerm {
    /// The term ID.
    pub term_id: String,
    /// The term name (optional).
    #[serde(default = "Option::default")]
    pub term_name: Option<String>,
}

/// Query result records.
pub mod query_result {
    use crate::common::Version;

    use super::HpoTerm;

    /// Struct for storing gene information in the result.
    #[derive(
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Debug,
        Clone,
        serde::Serialize,
        serde::Deserialize,
        utoipa::ToSchema,
    )]
    #[serde_with::skip_serializing_none]
    pub struct Gene {
        /// The NCBI gene ID.
        pub entrez_id: u32,
        /// The gene symbol.
        pub gene_symbol: String,
        /// The HGNC ID.
        pub hgnc_id: Option<String>,
    }

    /// The performed query.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[schema(title = "HpoSimTermGeneQuery")]
    pub struct Query {
        /// The query HPO terms.
        pub terms: Vec<HpoTerm>,
        /// The gene list to score.
        pub genes: Vec<Gene>,
    }

    /// Result container data structure.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[schema(title = "HpoSimTermGeneResult")]
    pub struct Result {
        /// Version information.
        pub version: Version,
        /// The original query records.
        pub query: Query,
        /// The resulting records for the scored genes.
        pub result: Vec<ResultEntry>,
    }

    /// Store score for a record with information on individual terms.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[schema(title = "HpoSimTermGeneResultEntry")]
    pub struct ResultEntry {
        /// The gene symbol.
        pub gene_symbol: String,
        /// The raw Phenomizer score.
        pub raw_score: f32,
        /// Details on individual terms.
        #[serde(default = "Option::default")]
        pub terms: Option<Vec<TermDetails>>,
    }

    /// Detailed term scores.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[schema(title = "HpoSimTermGeneTermDetails")]
    pub struct TermDetails {
        /// The query HPO term.
        pub term_query: Option<HpoTerm>,
        /// The gene's HPO term.
        pub term_gene: HpoTerm,
        /// The similarity score.
        pub score: f32,
    }
}

/// Run the actual phenotypic similarity query for patient terms and list of
/// genes.
///
/// # Arguments
///
/// * `patient`: The query/patient HPO terms.
/// * `genes`: The list of genes to score.
/// * `hpo`: The HPO ontology.
/// * `db`: The `RocksDB` instance for the Resnik P-values.
///
/// # Returns
///
/// * `Ok(query_result::Container)` if successful.
///
/// # Errors
///
/// In the case that there is a problem with query execution.
///
/// # Panics
///
/// In the case that a term or database lookup fails.
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_lines)]
pub fn run_query<S>(
    patient: &HpoGroup,
    genes: &Vec<&hpo::annotations::Gene>,
    hpo: &Ontology,
    ncbi_to_hgnc: &HashMap<u32, String, S>,
) -> Result<query_result::Result, anyhow::Error>
where
    S: std::hash::BuildHasher,
{
    let query = query_result::Query {
        terms: patient
            .iter()
            .map(|t| {
                let term = hpo.hpo(t).expect("could not resolve HPO term");
                HpoTerm {
                    term_id: term.id().to_string(),
                    term_name: Some(term.name().to_string()),
                }
            })
            .collect(),
        genes: Vec::new(),
    };
    let mut result = query_result::Result {
        version: crate::common::Version::new(&hpo.hpo_version()),
        query,
        result: Vec::new(),
    };
    for gene in genes {
        let ncbi_gene_id = gene.id().as_u32();
        tracing::debug!("gene = {:?}", gene);
        let raw_score = phenomizer::score(
            patient,
            &gene
                .to_hpo_set(hpo)
                .child_nodes()
                .without_modifier()
                .into_iter()
                .collect::<HpoGroup>(),
            hpo,
        );

        // For each term in the gene, provide query term with the highest similarity.
        let mut terms = gene
            .to_hpo_set(hpo)
            .child_nodes()
            .without_modifier()
            .into_iter()
            .collect::<HpoGroup>()
            .iter()
            .map(|gene_term_id| {
                let gene_term = hpo.hpo(gene_term_id).expect("gene HPO term not found");
                let (best_term, best_score) = patient
                    .iter()
                    .map(|query_term_id| {
                        let query_term = hpo.hpo(query_term_id).expect("query HPO term not found");
                        let score = gene_term.similarity_score(
                            &query_term,
                            &Builtins::Resnik(hpo::term::InformationContentKind::Gene),
                        );
                        (query_term, score)
                    })
                    .max_by(|(_, score1), (_, score2)| score1.partial_cmp(score2).unwrap())
                    .expect("could not determine best query term");

                let term_query = if best_score > 0.0 {
                    Some(HpoTerm {
                        term_id: best_term.id().to_string(),
                        term_name: Some(best_term.name().to_string()),
                    })
                } else {
                    None
                };

                TermDetails {
                    term_query,
                    term_gene: HpoTerm {
                        term_id: gene_term.id().to_string(),
                        term_name: Some(gene_term.name().to_string()),
                    },
                    score: best_score,
                }
            })
            .collect::<Vec<_>>();
        terms.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        result.query.genes.push(query_result::Gene {
            entrez_id: ncbi_gene_id,
            gene_symbol: gene.name().to_string(),
            hgnc_id: ncbi_to_hgnc.get(&ncbi_gene_id).cloned(),
        });

        result.result.push(query_result::ResultEntry {
            gene_symbol: gene.name().to_string(),
            raw_score,
            terms: Some(terms),
        });
    }

    // Sort genes for reproducibility.
    result.query.genes.sort();

    // Sort output records by raw score for reproducibility.
    result
        .result
        .sort_by(|a, b| b.raw_score.partial_cmp(&a.raw_score).unwrap());

    Ok(result)
}

/// Main entry point for `query` sub command.
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
    let before_loading = Instant::now();
    let hpo = crate::common::load_hpo(&args.path_hpo_dir)?;
    tracing::info!("...done loading HPO in {:?}", before_loading.elapsed());

    tracing::info!("Loading genes...");
    let before_load_genes = Instant::now();
    let genes_json = std::fs::read_to_string(&args.path_genes_json)?;
    let genes: Vec<Gene> = serde_json::from_str(&genes_json)?;
    let mut missing_genes = Vec::new();
    let genes = genes
        .iter()
        .filter_map(|g| {
            let mapped = hpo.gene_by_name(&g.gene_symbol);
            if mapped.is_none() {
                missing_genes.push(g.clone());
            }
            mapped
        })
        .collect::<Vec<_>>();
    tracing::info!("... done loadin genes in {:?}", before_load_genes.elapsed());

    tracing::info!("Loading (patient/query) HPO term ids...");
    let before_load_genes = Instant::now();
    let query_json = std::fs::read_to_string(&args.path_terms_json)?;
    let query: Vec<HpoTerm> = serde_json::from_str(&query_json)?;
    let query = query
        .iter()
        .map(|t| {
            HpoTermId::try_from(t.term_id.as_str())
                .unwrap_or_else(|_| panic!("term {} no valid HPO term ID", &t.term_id))
        })
        .collect::<Vec<_>>();
    let query = {
        let mut group = HpoGroup::new();
        for term in query {
            group.insert(term);
        }
        group
    };
    tracing::info!(
        "... done loading HPO IDs in {:?}",
        before_load_genes.elapsed()
    );

    tracing::info!("Loading HGNC xlink...");
    let before_load_xlink = Instant::now();
    let path_hgnc_xlink = format!("{}/hgnc_xlink.tsv", args.path_hpo_dir);
    let ncbi_to_hgnc = crate::common::hgnc_xlink::load_ncbi_to_hgnc(path_hgnc_xlink)?;
    tracing::info!(
        "... done loading HGNC xlink in {:?}",
        before_load_xlink.elapsed()
    );

    tracing::info!("Starting priorization...");
    let before_priorization = Instant::now();
    let result = run_query(&query, &genes, &hpo, &ncbi_to_hgnc)?;
    tracing::info!(
        "... done with prioritization in {:?}",
        before_priorization.elapsed()
    );

    println!("{result:#?}");

    tracing::info!(
        "{: >4} | {: <10} | {: >10} | {: >10}",
        "rank",
        "gene",
        "P-value",
        "score"
    );
    tracing::info!("     |            |");
    for (i, gene) in result.result.iter().enumerate() {
        tracing::info!(
            "{: >4} | {: <10} | {: >10.5}",
            i + 1,
            gene.gene_symbol,
            gene.raw_score
        );
    }

    tracing::info!("All done. Have a nice day!");
    Ok(())
}
