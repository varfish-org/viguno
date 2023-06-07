//! Implementation of `/hpo/genes`.

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{
    annotations::{AnnotationId, Gene, GeneId},
    Ontology,
};
use serde::{Deserialize, Serialize};

use crate::server::WebServerData;

use super::{CustomError, Match, ResultHpoTerm};

/// Parameters for `fetch_hpo_genes`.
///
/// This allows to query for genes.  The first given of the following is
/// interpreted.
///
/// - `gene_id` -- specify gene ID
/// - `gene_symbol` -- specify the gene symbol
/// - `max_results` -- the maximnum number of records to return
/// - `hpo_terms` -- whether to include `"hpo_terms"` in result
///
/// The following propery defines how matches are performed:
///
/// - `match` -- how to match
#[derive(Deserialize, Debug, Clone)]
struct Request {
    /// The gene ID to search for.
    pub gene_id: Option<String>,
    /// The gene symbol to search for.
    pub gene_symbol: Option<String>,
    /// The match mode.
    #[serde(alias = "match")]
    pub match_: Option<Match>,
    /// Maximal number of results to return.
    #[serde(default = "_default_max_results")]
    pub max_results: usize,
    /// Whether to include HPO terms.
    #[serde(default = "_default_hpo_terms")]
    pub hpo_terms: bool,
}

/// Return default of `Request::max_results`.
fn _default_max_results() -> usize {
    100
}

/// Return default of `Request::hpo_terms`.
fn _default_hpo_terms() -> bool {
    false
}

/// Result entry for `handle`.
#[derive(Serialize, Debug, Clone)]
struct ResultEntry {
    /// The gene's NCBI ID.
    pub gene_ncbi_id: u32,
    /// The gene's HGNC symbol.
    pub gene_symbol: String,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub hpo_terms: Option<Vec<ResultHpoTerm>>,
}

impl ResultEntry {
    pub fn from_gene_with_ontology(gene: &Gene, ontology: &Ontology, hpo_terms: bool) -> Self {
        let hpo_terms = if hpo_terms {
            Some(
                gene.to_hpo_set(ontology)
                    .child_nodes()
                    .into_iter()
                    .map(|term| ResultHpoTerm {
                        term_id: term.id().to_string(),
                        name: term.name().to_string(),
                    })
                    .collect(),
            )
        } else {
            None
        };
        ResultEntry {
            gene_ncbi_id: gene.id().as_u32(),
            gene_symbol: gene.name().to_string(),
            hpo_terms,
        }
    }
}

/// Query for genes in the HPO database.
#[allow(clippy::unused_async)]
#[get("/hpo/genes")]
async fn handle(
    data: Data<WebServerData>,
    _path: Path<()>,
    query: web::Query<Request>,
) -> actix_web::Result<impl Responder, CustomError> {
    let ontology = &data.ontology;
    let match_ = query.match_.unwrap_or_default();
    let mut result: Vec<ResultEntry> = Vec::new();

    if match_ == Match::Exact {
        let gene = if let Some(gene_ncbi_id) = &query.gene_id {
            let gene_id = GeneId::from(
                gene_ncbi_id
                    .parse::<u32>()
                    .map_err(|e| CustomError::new(anyhow::anyhow!(e)))?,
            );
            ontology.gene(&gene_id)
        } else if let Some(gene_symbol) = &query.gene_symbol {
            ontology.gene_by_name(gene_symbol)
        } else {
            None
        };
        if let Some(gene) = gene {
            result.push(ResultEntry::from_gene_with_ontology(
                gene,
                ontology,
                query.hpo_terms,
            ));
        }
    } else if let Some(gene_symbol) = &query.gene_symbol {
        let mut it = ontology.genes();
        let mut gene = it.next();
        while gene.is_some() && result.len() < query.max_results {
            let symbol = gene.expect("checked above").symbol();
            let is_match = match query.match_.unwrap_or_default() {
                Match::Prefix => symbol.starts_with(gene_symbol),
                Match::Suffix => symbol.ends_with(gene_symbol),
                Match::Contains => symbol.contains(gene_symbol),
                Match::Exact => panic!("cannot happen here"),
            };
            if is_match {
                result.push(ResultEntry::from_gene_with_ontology(
                    gene.expect("checked above"),
                    ontology,
                    query.hpo_terms,
                ));
            }

            gene = it.next();
        }
    }

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    #[test]
    fn test_handle() -> Result<(), anyhow::Error> {
        assert!(false, "actually write the test");

        Ok(())
    }
}
