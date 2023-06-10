//! Entry point `/hpo/sim/term-gene` that allows the similarity computation between a set of
//! terms and a gene.

use std::str::FromStr;

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};

use hpo::{annotations::GeneId, term::HpoGroup, HpoTermId, Ontology};

use super::super::CustomError;
use crate::{query, server::WebServerData};

/// Enum for representing similarity method to use.
#[derive(Default, Debug, Clone, Copy, derive_more::Display)]
pub enum SimilarityMethod {
    /// Phenomizer similarity score.
    #[default]
    #[display(fmt = "phenomizer")]
    Phenomizer,
}

impl FromStr for SimilarityMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "resnik::gene" => Self::Phenomizer,
            _ => anyhow::bail!("unknown similarity method: {}", s),
        })
    }
}

/// Parameters for `handle`.
///
/// This allows to compute differences between
///
/// - `terms` -- set of terms to use as query
/// - `gene_ids` -- set of ids for genes to use as "database"
/// - `gene_symbols` -- set of symbols for genes to use as
///   "database"
#[derive(serde::Deserialize, Debug, Clone)]
struct Request {
    /// Set of terms to use as query.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub terms: Vec<String>,
    /// The set of ids for genes to use as "database".
    #[serde(
        default = "Option::default",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::super::option_vec_str_deserialize"
    )]
    pub gene_ids: Option<Vec<String>>,
    /// The set of symbols for genes to use as "database".
    #[serde(
        default = "Option::default",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::super::option_vec_str_deserialize"
    )]
    pub gene_symbols: Option<Vec<String>>,
    /// The similarity method to use.
    #[serde(
        deserialize_with = "help::similarity_deserialize",
        default = "help::default_sim"
    )]
    pub sim: SimilarityMethod,
}

/// Helpers for deserializing `Request`.
mod help {
    /// Helper to deserialize a similarity method.
    pub fn similarity_deserialize<'de, D>(
        deserializer: D,
    ) -> Result<super::SimilarityMethod, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }

    /// Default value for `Request::sim`.
    pub fn default_sim() -> super::SimilarityMethod {
        super::SimilarityMethod::Phenomizer
    }
}

/// Query for similarity between a set of terms to each entry in a
/// list of genes.
#[allow(clippy::unused_async)]
#[get("/hpo/sim/term-gene")]
async fn handle(
    data: Data<WebServerData>,
    _path: Path<()>,
    query: web::Query<Request>,
) -> actix_web::Result<impl Responder, CustomError> {
    let _ = &query.sim;

    let hpo: &Ontology = &data.ontology;

    // Translate strings from the query into an `HpoGroup`.
    let query_terms = {
        let mut query_terms = HpoGroup::new();
        for term in &query.terms {
            if let Some(term) = hpo.hpo(HpoTermId::from(term.clone())) {
                query_terms.insert(term.id());
            }
        }
        query_terms
    };

    // Translate strings from the query into genes via symbol or gene ID.
    let genes = if let Some(gene_ids) = &query.gene_ids {
        Ok(gene_ids
            .iter()
            .filter_map(|gene_id| {
                if let Ok(gene_id) = gene_id.parse::<u32>() {
                    hpo.gene(&GeneId::from(gene_id))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    } else if let Some(gene_symbols) = &query.gene_symbols {
        Ok(gene_symbols
            .iter()
            .filter_map(|gene_symbol| hpo.gene_by_name(gene_symbol))
            .collect::<Vec<_>>())
    } else {
        Err(CustomError::new(anyhow::anyhow!(
            "either `gene_ids` or `gene_symbols` must be given"
        )))
    }?;

    // Perform similarity computation.
    let result = query::run_query(
        &query_terms,
        &genes,
        hpo,
        data.db.as_ref().expect("must provide RocksDB"),
    )
    .map_err(CustomError::new)?;

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    /// Helper function for running a query.
    #[allow(dead_code)]
    async fn run_query(uri: &str) -> Result<crate::query::query_result::Container, anyhow::Error> {
        let hpo_path = "tests/data/hpo";
        let ontology = crate::common::load_hpo(hpo_path)?;
        let db = Some(rocksdb::DB::open_cf_for_read_only(
            &rocksdb::Options::default(),
            format!("{}/{}", hpo_path, "resnik"),
            ["meta", "resnik_pvalues"],
            true,
        )?);

        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(crate::server::WebServerData {
                    ontology,
                    db,
                }))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: crate::query::query_result::Container =
            actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[actix_web::test]
    async fn hpo_sim_term_gene_terms_gene_ids() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/sim/term-gene?terms=HP:0010442,HP:0000347&gene_ids=23483,7273&sim=resnik::gene")
                .await?
        ))
    }

    #[actix_web::test]
    async fn hpo_sim_term_gene_terms_symbols() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/sim/term-gene?terms=HP:0010442,HP:0000347&gene_symbols=TGDS,TTN&sim=resnik::gene")
                .await?
        ))
    }
}
