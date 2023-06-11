//! Implementation of the Actix server.

pub mod hpo_genes;
pub mod hpo_omims;
pub mod hpo_sim;
pub mod hpo_terms;

use actix_web::{middleware::Logger, web::Data, App, HttpServer, ResponseError};
use serde::{Deserialize, Deserializer, Serialize};

use super::{Args, WebServerData};

#[derive(Debug)]
struct CustomError {
    err: anyhow::Error,
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.err)
    }
}

impl CustomError {
    fn new(err: anyhow::Error) -> Self {
        CustomError { err }
    }
}

impl ResponseError for CustomError {}

/// Specify how to perform query matches in the API calls.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Match {
    #[default]
    Exact,
    Prefix,
    Suffix,
    Contains,
}

/// Representation of a gene.
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[serde_with::skip_serializing_none]
struct ResultGene {
    /// The HPO ID.
    pub ncbi_gene_id: u32,
    /// The description.
    pub gene_symbol: String,
    /// The HGNC ID.
    pub hgnc_id: Option<String>,
}

/// Representation of an HPO term.
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
struct ResultHpoTerm {
    /// The HPO ID.
    pub term_id: String,
    /// The description.
    pub name: String,
}

/// Helper to deserialize a comma-separated list of strings.
fn vec_str_deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    Ok(str_sequence
        .split(',')
        .map(std::borrow::ToOwned::to_owned)
        .collect())
}

/// Helper to deserialize a comma-separated list of strings.
fn option_vec_str_deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    if str_sequence.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            str_sequence
                .split(',')
                .map(std::borrow::ToOwned::to_owned)
                .collect(),
        ))
    }
}

/// Main entry point for running the REST server.
#[allow(clippy::unused_async)]
#[actix_web::main]
pub async fn main(args: &Args, dbs: Data<WebServerData>) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(dbs.clone())
            .service(hpo_genes::handle)
            .service(hpo_terms::handle)
            .service(hpo_omims::handle)
            .service(hpo_sim::term_term::handle)
            .service(hpo_sim::term_gene::handle)
            .wrap(Logger::default())
    })
    .bind((args.listen_host.as_str(), args.listen_port))?
    .run()
    .await
}
