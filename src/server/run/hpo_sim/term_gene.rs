//! Entry point `/hpo/sim/term-gene` that allows the similarity computation between a set of
//! terms and a gene.

use std::sync::Arc;

use actix_web::{
    get,
    web::{self, Data, Json, Path},
};

use hpo::{annotations::GeneId, term::HpoGroup, HpoTermId, Ontology};

use super::super::CustomError;
use crate::{
    query::{self, query_result::HpoSimTermGeneResult},
    server::run::WebServerData,
};

/// Parameters for `handle`.
///
/// This allows to compute differences between
///
/// - `terms` -- set of terms to use as query
/// - `gene_ids` -- set of ids for genes to use as "database", can be NCBI\
///                 gene ID or HGNC gene ID.
/// - `gene_symbols` -- set of symbols for genes to use as
///   "database"
#[derive(serde::Deserialize, Debug, Clone, utoipa::ToSchema, utoipa::IntoParams)]
pub struct HpoSimTermGeneQuery {
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
}

/// Query for similarity between a set of terms to each entry in a
/// list of genes.
#[allow(clippy::unused_async)]
#[utoipa::path(
    get,
    operation_id = "hpoSimTermGene",
    params(HpoSimTermGeneQuery),
    responses(
        (status = 200, description = "The query was successful.", body = HpoSimTermGeneResult),
        (status = 500, description = "The server encountered an error.", body = CustomError)
    )
)]
#[get("/api/v1/hpo/sim/term-gene")]
async fn handle(
    data: Data<Arc<WebServerData>>,
    _path: Path<()>,
    query: web::Query<HpoSimTermGeneQuery>,
) -> actix_web::Result<Json<HpoSimTermGeneResult>, CustomError> {
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
                } else if let Some(gene_id) = data.hgnc_to_ncbi.get(gene_id) {
                    hpo.gene(&GeneId::from(*gene_id))
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
    let result = query::run_query(&query_terms, &genes, hpo, &data.ncbi_to_hgnc)
        .map_err(CustomError::new)?;

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::server::run::hpo_genes::test::web_server_data;

    /// Helper function for running a query.
    #[allow(dead_code)]
    pub async fn run_query(
        web_server_data: Arc<crate::server::run::WebServerData>,
        uri: &str,
    ) -> Result<crate::query::query_result::HpoSimTermGeneResult, anyhow::Error> {
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(web_server_data))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: crate::query::query_result::HpoSimTermGeneResult =
            actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_sim_term_gene_terms_ncbi_gene_ids(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/sim/term-gene?terms=HP:0010442,HP:0000347&gene_ids=23483,7273"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_sim_term_gene_terms_hgnc_gene_ids(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/sim/term-gene?terms=HP:0010442,HP:0000347&gene_ids=HGNC:20324,HGNC:12403"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_sim_term_gene_terms_symbols(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/sim/term-gene?terms=HP:0010442,HP:0000347&gene_symbols=TGDS,TTN"
            )
            .await?
        ))
    }
}
