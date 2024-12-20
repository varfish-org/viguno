//! Implementation of `/hpo/genes`.

use std::{collections::HashMap, sync::Arc};

use actix_web::{
    get,
    web::{self, Data, Json, Path},
};
use hpo::{
    annotations::{AnnotationId, Gene, GeneId},
    Ontology,
};

use crate::{common::Version, server::run::WebServerData};

use super::{CustomError, Match, ResultHpoTerm};

/// Parameters for `fetch_hpo_genes`.
///
/// This allows to query for genes.  The first given of the following is
/// interpreted.
///
/// - `gene_id` -- specify gene ID (either NCBI or HGNC gene ID)
/// - `gene_symbol` -- specify the gene symbol
/// - `max_results` -- the maximnum number of records to return
/// - `hpo_terms` -- whether to include `"hpo_terms"` in result
///
/// The following propery defines how matches are performed:
///
/// - `match` -- how to match
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams,
)]
pub struct HpoGenesQuery {
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
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
)]
pub struct HpoGenesResultEntry {
    /// The gene's NCBI ID.
    pub gene_ncbi_id: u32,
    /// The gene's HGNC symbol.
    pub gene_symbol: String,
    /// The gene's HGNC ID.
    pub hgnc_id: Option<String>,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub hpo_terms: Option<Vec<ResultHpoTerm>>,
}

impl HpoGenesResultEntry {
    /// Create a `ResultEntry` from a `Gene` with an `Ontology`.
    pub fn from_gene_with_ontology(
        gene: &Gene,
        ontology: &Ontology,
        hpo_terms: bool,
        ncbi_to_hgnc: &HashMap<u32, String>,
    ) -> Self {
        let hpo_terms = if hpo_terms {
            let mut terms = gene
                .to_hpo_set(ontology)
                .child_nodes()
                .into_iter()
                .map(|term| ResultHpoTerm {
                    term_id: term.id().to_string(),
                    name: term.name().to_string(),
                })
                .collect::<Vec<_>>();
            terms.sort();
            Some(terms)
        } else {
            None
        };
        HpoGenesResultEntry {
            gene_ncbi_id: gene.id().as_u32(),
            gene_symbol: gene.name().to_string(),
            hgnc_id: ncbi_to_hgnc.get(&gene.id().as_u32()).cloned(),
            hpo_terms,
        }
    }
}

/// Container for the result.
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct HpoGenesResult {
    /// Version information.
    pub version: Version,
    /// The original query records.
    pub query: HpoGenesQuery,
    /// The resulting records for the scored genes.
    pub result: Vec<HpoGenesResultEntry>,
}

/// Query for genes in the HPO database.
///
/// # Errors
///
/// In the case that there is an error running the server.
#[utoipa::path(
    get,
    operation_id = "hpoGenes",
    params(HpoGenesQuery),
    responses(
        (status = 200, description = "The query was successful.", body = HpoGenesResult),
        (status = 500, description = "The server encountered an error.", body = CustomError)
    )
)]
#[get("/api/v1/hpo/genes")]
async fn handle(
    data: Data<Arc<WebServerData>>,
    _path: Path<()>,
    query: web::Query<HpoGenesQuery>,
) -> actix_web::Result<Json<HpoGenesResult>, CustomError> {
    let ontology = &data.ontology;
    let match_ = query.match_.unwrap_or_default();
    let mut result: Vec<HpoGenesResultEntry> = Vec::new();

    if match_ == Match::Exact {
        let gene = if let Some(gene_id) = &query.gene_id {
            let gene_id = if let Ok(ncbi_gene_id) = gene_id.parse::<u32>() {
                Ok(GeneId::from(ncbi_gene_id))
            } else if let Some(ncbi_gene_id) = data.hgnc_to_ncbi.get(gene_id) {
                Ok(GeneId::from(*ncbi_gene_id))
            } else {
                Err(CustomError::new(anyhow::anyhow!("could not parse gene ID")))
            }?;
            ontology.gene(&gene_id)
        } else if let Some(gene_symbol) = &query.gene_symbol {
            ontology.gene_by_name(gene_symbol)
        } else {
            None
        };
        if let Some(gene) = gene {
            result.push(HpoGenesResultEntry::from_gene_with_ontology(
                gene,
                ontology,
                query.hpo_terms,
                &data.ncbi_to_hgnc,
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
                result.push(HpoGenesResultEntry::from_gene_with_ontology(
                    gene.expect("checked above"),
                    ontology,
                    query.hpo_terms,
                    &data.ncbi_to_hgnc,
                ));
            }

            gene = it.next();
        }
    }

    result.sort();

    let result = HpoGenesResult {
        version: Version::new(&data.ontology.hpo_version()),
        query: query.into_inner(),
        result,
    };

    Ok(Json(result))
}

#[cfg(test)]
pub(crate) mod test {
    use std::sync::Arc;

    /// Return the default ``crate::server::run::WebServerData`` for testing.
    #[rstest::fixture]
    #[once]
    pub fn web_server_data() -> Arc<crate::server::run::WebServerData> {
        let ontology = crate::common::load_hpo("tests/data/hpo").expect("could not load HPO");
        let ncbi_to_hgnc =
            crate::common::hgnc_xlink::load_ncbi_to_hgnc("tests/data/hpo/hgnc_xlink.tsv")
                .expect("could not HGNC xlink");
        let hgnc_to_ncbi = crate::common::hgnc_xlink::inverse_hashmap(&ncbi_to_hgnc);
        let hpo_doc = fastobo::from_file("tests/data/hpo/hp.obo").expect("could not load HPO OBO");

        Arc::new(crate::server::run::WebServerData {
            ontology,
            ncbi_to_hgnc,
            hgnc_to_ncbi,
            full_text_index: crate::index::Index::new(hpo_doc)
                .expect("could not create full text index"),
        })
    }

    /// Helper function for running a query.
    #[allow(dead_code)]
    pub async fn run_query(
        web_server_data: Arc<crate::server::run::WebServerData>,
        uri: &str,
    ) -> Result<super::HpoGenesResult, anyhow::Error> {
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(web_server_data))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: super::HpoGenesResult = actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_ncbi_gene_id_exact_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(web_server_data.clone(), "/api/v1/hpo/genes?gene_id=2348").await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_ncbi_gene_id_exact_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_id=2348&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_hgnc_gene_id_exact_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_id=HGNC:3791"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_hgnc_gene_id_exact_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_id=HGNC:3791&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_exact_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=TGDS"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_exact_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=TGDS&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_prefix_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=TGD&match=prefix"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_prefix_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=TGD&match=prefix&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_suffix_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=GDS&match=suffix"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_suffix_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=GDS&match=suffix&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_contains_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=GD&match=contains"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_genes_gene_symbol_contains_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/genes?gene_symbol=GD&match=contains&hpo_terms=true"
            )
            .await?
        ))
    }
}
